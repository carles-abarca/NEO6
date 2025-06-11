use regex::Regex;

fn main() {
    let color_start_regex = Regex::new(r"\[(BLUE|RED|PINK|GREEN|TURQUOISE|YELLOW|WHITE|DEFAULT)\]").unwrap();
    
    let test_cases = vec![
        "[BLUE]|[/BLUE][XY2,25]",
        "[BLUE]text[/BLUE]",
        "[YELLOW]text[/YELLOW]",
    ];
    
    for test in test_cases {
        println!("Testing: {}", test);
        if let Some(cap) = color_start_regex.captures(test) {
            println!("  Matched: {:?}", cap);
            println!("  Color: {}", &cap[1]);
        } else {
            println!("  No match");
        }
        println!();
    }
}
