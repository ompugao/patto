# Patto Note Syntax Guide for AI Content Conversion

**Patto Note 🪽** is a line-oriented, indentation-sensitive plain-text format designed for rapid note-taking, outlining, and task management. It is inspired by Scrapbox and optimized for hierarchy and linking.

## 1. Core Principles
- **Line-oriented**: Every line is a distinct unit. Newlines represent new lines or items.
- **Indentation Hierarchy**: Use **Tabs (`\t`)** for nesting. A line starting with one or more tabs is a child of the preceding line with fewer tabs.
- **Wikilink First**: Knowledge is connected via bracketed links `[note name]`.

## 2. Basic Formatting
| Feature | Syntax | Notes |
| :--- | :--- | :--- |
| **Hierarchy** | `\tContent` | One tab per level. |
| **Bold** | `[* Text]` | Can use multiple stars for size: `[*** Large Bold]`. |
| **Italic** | `[/ Text]` | |
| **Underline** | `[_ Text]` | |
| **Strike-through** | `[- Text]` | |
| **Combined** | `[*/ Bold Italic]` | Multiple symbols can be combined in one bracket. |
| **Inline Code** | `[` code `]` | Backticks inside brackets. |
| **Inline Math** | `[$ \sum_i x_i $]` | LaTeX-style math inside `[$ $]`. |
| **Horizontal Line** | `-----` | 5 or more dashes. |

## 3. Links & Wiki Syntax
- **Internal Link**: `[Other Note Name]`
- **Anchored Link**: `[Note Name#Anchor-Name]` (Links to a specific line).
- **External Link (URL Title)**: `[https://example.com Title]` or `[Title https://example.com]`
- **Email**: `[mailto:user@example.com Label]`

## 4. Task Management
Tasks can be defined using symbols or properties at the start or end of a line.
- **Todo**: `Task description !2024-12-31`
- **Doing**: `Task description *2024-12-31`
- **Done**: `Task description -2024-12-31`
- **Property-based**: `Task description {@task status=todo due=2024-12-31}`
    - Status values: `todo`, `doing` (or `inprogress`, `wip`), `done`.
    - Due format: `YYYY-MM-DD` or `YYYY-MM-DDTHH:MM`.

## 5. Blocks
Blocks start with `[@type]` and their content **must be indented** on the following lines.
- **Code Block**:
  ```patto
  [@code python]
  	print("Hello World")
  ```
- **Quote Block**:
  ```patto
  [@quote]
  	This is a quoted line.
  	It can contain links like [Example].
  ```
- **Table Block**: Uses tabs to separate columns.
  ```patto
  [@table caption="Comparison"]
  	Header 1	Header 2
  	Row 1 Col 1	Row 1 Col 2
  ```
- **Math Block**:
  ```patto
  [@math]
  	f(x) = \int_{-\infty}^{\infty} e^{-x^2} dx
  ```

## 6. Embeddings
Embeddings follow the `[@tag src "alt/title"]` pattern.
- **Images**: `[@img http://example.com/a.png "Alt Text"]`
    - Local images **must** use `./` or `../`: `[@img ./assets/img.png]`
- **General Embeds**: `[@embed https://youtube.com/... "Video Title"]`
    - Supports YouTube, Twitter (X), SpeakerDeck, SlideShare.
    - **PDF**: `[@embed ./doc.pdf "Title"]`

## 7. Anchors & Properties
- **Anchors**: `#my-anchor` or `{@anchor my-anchor}`. Used for linking to specific lines.
- **Custom Properties**: `{@key value}` or `{@key k1=v1 k2=v2}`.

## 8. Structural Rules for Conversion
1. **Never use Markdown headers (`#`, `##`)**. Use indentation and `[* bold]` for emphasis.
2. **Lists**: Do not use `-` or `*` as bullets (these are for tasks). Use plain text with tabs.
3. **Hierarchy**: If converting a bulleted list, convert `- item` to `\titem`.
4. **Task Symbols**: Always ensure the date follows `!`, `*`, or `-` immediately without a space if it's a task.
5. **Indentation in Blocks**: Content inside `[@code]`, `[@quote]`, `[@table]`, or `[@math]` must be indented exactly one level deeper than the `[@...]` command line.

## 9. Full Example Document
```patto
Patto Note Project Overview
	[Goals]
		Create a simpler alternative to [Markdown]
		Focus on [outlining] and [task management]

	Architecture
		[@code rust]
			fn main() {
				println!("Line-oriented power!");
			}
		
	Current Tasks
		Finish parser implementation #milestone-1 !2024-05-01
		Refactor LSP backend *2024-05-10
		Define initial syntax spec -2024-04-20

	Resources
		[@img ./assets/logo.png "Patto Logo"]
		[@embed https://www.youtube.com/watch?v=dQw4w9WgXcQ "Intro Video"]

	Comparison Table
		[@table]
			Feature	Markdown	Patto
			Nesting	Mixed	Strict (Tabs)
			Linking	Complex	Simple [brackets]
			Tasks	Plugin-dep	Native Support
```
