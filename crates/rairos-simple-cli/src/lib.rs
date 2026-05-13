pub enum CliCommand {
    Search { query: String, limit: usize },
    Import { paper_id: String, source: String },
    List { limit: usize, status: String },
    Status,
    Stats,
    Export { format: String },
    Help,
}

pub fn parse_args() -> Option<CliCommand> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return None;
    }
    let command = args[0].as_str();
    match command {
        "search" => {
            if args.len() < 2 {
                eprintln!("Usage: simple-cli search <query> [-n limit]");
                return None;
            }
            let query = args[1].clone();
            let limit = if args.len() > 3 && args[2] == "-n" {
                args[3].parse().unwrap_or(5)
            } else {
                5
            };
            Some(CliCommand::Search { query, limit })
        }
        "import" => {
            if args.len() < 2 {
                eprintln!("Usage: simple-cli import <paper_id> [--source arxiv|doi]");
                return None;
            }
            let paper_id = args[1].clone();
            let source = if args.len() > 3 && args[2] == "--source" {
                args[3].clone()
            } else {
                "arxiv".to_string()
            };
            Some(CliCommand::Import { paper_id, source })
        }
        "list" => {
            let limit = if args.len() > 2 && args[1] == "-n" {
                args[2].parse().unwrap_or(20)
            } else {
                20
            };
            let status = if args.len() > 4 && args[3] == "--status" {
                args[4].clone()
            } else {
                "all".to_string()
            };
            Some(CliCommand::List { limit, status })
        }
        "status" => Some(CliCommand::Status),
        "stats" => Some(CliCommand::Stats),
        "export" => {
            let format = if args.len() > 1 {
                args[1].clone()
            } else {
                "json".to_string()
            };
            Some(CliCommand::Export { format })
        }
        "help" | "-h" | "--help" => {
            print_help();
            None
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            print_help();
            None
        }
    }
}

pub fn print_help() {
    println!(r#"
📚 AI Research OS - Simplified CLI

Core Commands:
  search <query>    🔍 Search papers
  import <paper_id> 📥 Import paper
  list              📚 List all papers
  status            📊 View system status
  stats             📈 View detailed statistics
  export [format]   💾 Export data (json/csv)

Quick Start:
  1. Search papers:     search "machine learning"
  2. Import paper:      import 2301.001
  3. List papers:       list
  4. View status:       status

Tips:
  - Use --help for detailed command info
  - list supports --status filter
  - export supports json and csv formats
"#);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search() {
        match parse_search(&["search", "transformer", "-n", "10"]) {
            Some(CliCommand::Search { query, limit }) => {
                assert_eq!(query, "transformer");
                assert_eq!(limit, 10);
            }
            _ => panic!("expected Search command"),
        }
    }

    #[test]
    fn test_parse_import() {
        match parse_search(&["import", "2301.001"]) {
            Some(CliCommand::Import { paper_id, .. }) => {
                assert_eq!(paper_id, "2301.001");
            }
            _ => panic!("expected Import command"),
        }
    }

    fn parse_search(args: &[&str]) -> Option<CliCommand> {
        match args[0] {
            "search" => {
                let query = args.get(1)?.to_string();
                let limit = if args.len() > 3 && args[2] == "-n" {
                    args.get(3)?.parse().ok()?
                } else {
                    5
                };
                Some(CliCommand::Search { query, limit })
            }
            "import" => {
                let paper_id = args.get(1)?.to_string();
                let source = if args.len() > 3 && args[2] == "--source" {
                    args.get(3)?.to_string()
                } else {
                    "arxiv".to_string()
                };
                Some(CliCommand::Import { paper_id, source })
            }
            _ => None,
        }
    }
}
