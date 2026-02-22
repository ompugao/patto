use patto::parser::parse_text;

fn main() {
    let text1 = "相手のやりたいことを知り、働き方を改善し、[* 生産性を高められる]ようにする";
    println!("Text 1:\n{}", serde_json::to_string_pretty(&parse_text(text1).ast).unwrap());
}
