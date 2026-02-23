use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use clap::Parser as ClapParser;

#[derive(ClapParser)]
#[command(version, about, long_about=None)]
struct Cli {
    /// an input file to parse
    #[arg(short, long, value_name = "FILE")]
    file: PathBuf,
    /// an output html file
    #[arg(short, long, value_name = "OUTPUT")]
    output: PathBuf,
    /// theme, light or dark
    #[arg(short, long, value_name = "THEME")]
    theme: String,
    /// generate PDF output (requires chromium)
    #[cfg(feature = "pdf")]
    #[arg(long, value_name = "PDF_OUTPUT")]
    pdf: Option<PathBuf>,

    /// debug log file
    #[arg(short, long, value_name = "FILE")]
    debuglogfile: Option<PathBuf>,
    #[command(flatten)]
    verbose: Verbosity<InfoLevel>,
}

use patto::parser;
use patto::renderer;
use patto::renderer::Renderer;

use clap_verbosity_flag::{InfoLevel, Verbosity};
use std::fs::File;

const PATTO_CSS: &str = include_str!("../../assets/patto-html.css");

fn init_logger(filter_level: log::LevelFilter, logfile: Option<PathBuf>) {
    let mut loggers = Vec::new();
    if let Some(filename) = logfile {
        loggers.push(simplelog::WriteLogger::new(
            filter_level,
            simplelog::Config::default(),
            File::create(filename).unwrap(),
        ) as Box<dyn simplelog::SharedLogger>)
    }
    simplelog::CombinedLogger::init(loggers).unwrap();
}

fn render_html(
    text: &str,
    theme: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let parser::ParserResult {
        ast: rootnode,
        parse_errors: _,
    } = parser::parse_text(text);

    let options = renderer::HtmlRendererOptions {};
    let renderer = renderer::HtmlRenderer::new(options);

    let theme_class = match theme {
        "dark" => "theme-dark",
        _ => "theme-light",
    };
    let hljs_theme = if theme == "dark" { "-dark" } else { "" };

    let mut buf = Vec::new();
    write!(
        buf,
        r#"<!DOCTYPE html>
<html lang="en" class="{theme_class}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<style>
{PATTO_CSS}
</style>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github{hljs_theme}.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script>hljs.highlightAll();</script>
<script id="MathJax-script" async src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
</head>
<body>
<script type="module">
import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';
mermaid.initialize({{ startOnLoad: true, theme: 'forest' }});
</script>
<div class="patto-container">
"#
    )?;
    renderer.format(&rootnode, &mut buf)?;
    write!(buf, "</div>\n</body>\n</html>\n")?;
    Ok(String::from_utf8(buf)?)
}

#[cfg(feature = "pdf")]
async fn generate_pdf(
    html_content: &str,
    pdf_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use chromiumoxide::browser::{Browser, BrowserConfig};
    use chromiumoxide::cdp::browser_protocol::page::PrintToPdfParams;
    use futures::StreamExt;

    let tmp_dir = tempfile::tempdir()?;
    let html_path = tmp_dir.path().join("patto_output.html");
    fs::write(&html_path, html_content)?;

    let (mut browser, mut handler) = Browser::launch(
        BrowserConfig::builder()
            .no_sandbox()
            .build()
            .map_err(|e| format!("Failed to build browser config: {}", e))?,
    )
    .await
    .map_err(|e| format!("Failed to launch browser: {}", e))?;

    let handle = tokio::spawn(async move {
        while let Some(h) = handler.next().await {
            if h.is_err() {
                break;
            }
        }
    });

    let file_url = format!("file://{}", html_path.canonicalize()?.display());
    let page = browser
        .new_page(&file_url)
        .await
        .map_err(|e| format!("Failed to open page: {}", e))?;

    // Wait for the page to fully render
    page.wait_for_navigation()
        .await
        .map_err(|e| format!("Navigation error: {}", e))?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let pdf_params = PrintToPdfParams::builder()
        .print_background(true)
        .prefer_css_page_size(false)
        .margin_top(0.4)
        .margin_bottom(0.4)
        .margin_left(0.4)
        .margin_right(0.4)
        .build();

    let pdf_data = page
        .pdf(pdf_params)
        .await
        .map_err(|e| format!("Failed to generate PDF: {}", e))?;

    fs::write(pdf_path, pdf_data)?;

    browser.close().await.map_err(|e| format!("Failed to close browser: {}", e))?;
    handle.await?;
    Ok(())
}

#[cfg(feature = "pdf")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    init_logger(args.verbose.log_level_filter(), args.debuglogfile);

    let text = fs::read_to_string(&args.file).expect("cannot read input file");
    let html_content = render_html(&text, &args.theme)?;

    // Write HTML output
    let mut writer = BufWriter::new(fs::File::create(&args.output)?);
    write!(writer, "{}", html_content)?;
    writer.flush()?;

    // Generate PDF if requested
    if let Some(pdf_path) = &args.pdf {
        eprintln!("Generating PDF to {}...", pdf_path.display());
        generate_pdf(&html_content, pdf_path).await?;
        eprintln!("PDF generated successfully.");
    }

    Ok(())
}

#[cfg(not(feature = "pdf"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    init_logger(args.verbose.log_level_filter(), args.debuglogfile);

    let text = fs::read_to_string(&args.file).expect("cannot read input file");
    let html_content = render_html(&text, &args.theme)?;

    let mut writer = BufWriter::new(fs::File::create(&args.output)?);
    write!(writer, "{}", html_content)?;
    Ok(())
}
