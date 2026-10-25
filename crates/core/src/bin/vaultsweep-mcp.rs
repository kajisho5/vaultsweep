//! vaultsweep-mcp
//!   （引数なし / --mcp）  MCP サーバーとして標準入出力で動く
//!   --scan               診断して、まとめを JSON で出す
//!   --version

use vaultsweep_core::mcp::{Scanner, Server};
use vaultsweep_core::model::Report;
use vaultsweep_core::net::{Fetcher, HttpFetcher};
use vaultsweep_core::op::RealOp;
use vaultsweep_core::scan;
use vaultsweep_core::store::Store;

struct RealScanner;

impl Scanner for RealScanner {
    fn scan(&self, store: &Store) -> Result<Report, String> {
        let op = RealOp::default();
        let f = HttpFetcher::new().ok();
        let r = scan::run(&op, f.as_ref().map(|x| x as &dyn Fetcher), store, &|_| {})
            .map_err(|e| e.to_string())?;
        Ok(r.report)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let store = Store::open_default();
    match args.first().map(|s| s.as_str()) {
        Some("--version") => println!("vaultsweep-mcp {}", env!("CARGO_PKG_VERSION")),
        Some("--scan") => match RealScanner.scan(&store) {
            Ok(r) => println!(
                "{}",
                serde_json::json!({ "score": r.score, "total": r.total, "with_issues": r.flagged_count(), "scanned_at": r.scanned_at })
            ),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        },
        None | Some("--mcp") => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            let mut s = Server::new(&store, &RealScanner);
            if let Err(e) = s.serve(stdin.lock(), stdout.lock()) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        Some(other) => {
            eprintln!(
                "unknown option: {other}\nusage: vaultsweep-mcp [--mcp | --scan | --version]"
            );
            std::process::exit(2);
        }
    }
}
