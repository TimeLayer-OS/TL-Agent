//! tl-agent — CLI for TimeLayer Receipt-Gated Agent SDK
//!
//! Usage:
//!   tl-agent check  <bundle_dir> <action_id> [--verifier <path>]
//!   tl-agent next   <bundle_dir> <action_id> [--verifier <path>]
//!   tl-agent audit  <bundle_dir>             [--verifier <path>]
//!   tl-agent record <bundle_dir> <action_id> <result_hash> [--verifier <path>]
//!
//! Exit codes:
//!   0 — ALLOW / success
//!   1 — STOP / error

use std::path::PathBuf;
use std::process;

use timelayer_agent_sdk::{AgentBundle, CheckResult};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "check"  => cmd_check(&args),
        "next"   => cmd_next(&args),
        "audit"  => cmd_audit(&args),
        "record" => cmd_record(&args),
        "help" | "--help" | "-h" => { print_usage(); process::exit(0); }
        other => {
            eprintln!("tl-agent: unknown command '{other}'");
            print_usage();
            process::exit(1);
        }
    }
}

// ── check ──────────────────────────────────────────────────────────────────

fn cmd_check(args: &[String]) {
    // tl-agent check <bundle_dir> <action_id> [--verifier <path>]
    if args.len() < 4 {
        eprintln!("usage: tl-agent check <bundle_dir> <action_id> [--verifier <path>]");
        process::exit(1);
    }
    let bundle_dir = PathBuf::from(&args[2]);
    let action_id  = &args[3];
    let verifier   = parse_verifier_flag(args);

    let bundle = load_bundle(&bundle_dir, verifier.as_deref());

    match bundle.check_action(action_id) {
        CheckResult::Allow { action_id, receipt_id, allowed_next } => {
            println!("ALLOW");
            println!("  action  : {action_id}");
            println!("  receipt : {receipt_id}");
            if allowed_next.is_empty() {
                println!("  next    : (none — terminal action)");
            } else {
                println!("  next    : {}", allowed_next.join(", "));
            }
            process::exit(0);
        }
        CheckResult::Stop { action_id, reason, message } => {
            println!("STOP");
            println!("  action  : {action_id}");
            println!("  reason  : {reason}");
            println!("  message : {message}");
            process::exit(1);
        }
    }
}

// ── next ───────────────────────────────────────────────────────────────────

fn cmd_next(args: &[String]) {
    // tl-agent next <bundle_dir> <action_id> [--verifier <path>]
    if args.len() < 4 {
        eprintln!("usage: tl-agent next <bundle_dir> <action_id> [--verifier <path>]");
        process::exit(1);
    }
    let bundle_dir = PathBuf::from(&args[2]);
    let action_id  = &args[3];
    let verifier   = parse_verifier_flag(args);

    let bundle = load_bundle(&bundle_dir, verifier.as_deref());
    let next   = bundle.allowed_next(action_id);

    if next.is_empty() {
        println!("(none — '{action_id}' is a terminal action or not in topology)");
    } else {
        for a in &next {
            println!("{a}");
        }
    }
    process::exit(0);
}

// ── audit ──────────────────────────────────────────────────────────────────

fn cmd_audit(args: &[String]) {
    // tl-agent audit <bundle_dir> [--verifier <path>]
    if args.len() < 3 {
        eprintln!("usage: tl-agent audit <bundle_dir> [--verifier <path>]");
        process::exit(1);
    }
    let bundle_dir = PathBuf::from(&args[2]);
    let verifier   = parse_verifier_flag(args);

    let bundle  = load_bundle(&bundle_dir, verifier.as_deref());
    let entries = bundle.audit();

    let total   = entries.len();
    let valid   = entries.iter().filter(|e| e.is_valid()).count();
    let invalid = total - valid;

    println!("=== tl-agent audit ===");
    println!("bundle  : {}", bundle_dir.display());
    println!("actions : {total}  valid: {valid}  stop: {invalid}");
    println!();

    for entry in &entries {
        match &entry.result {
            CheckResult::Allow { receipt_id, allowed_next, .. } => {
                println!("  [ALLOW] {}", entry.action_id);
                println!("          receipt: {receipt_id}");
                if !allowed_next.is_empty() {
                    println!("          next   : {}", allowed_next.join(", "));
                }
            }
            CheckResult::Stop { reason, message, .. } => {
                println!("  [STOP]  {}", entry.action_id);
                println!("          reason : {reason}");
                println!("          msg    : {message}");
            }
        }
    }

    println!();
    if invalid == 0 {
        println!("result: ALL VALID — bundle is ready for use");
        process::exit(0);
    } else {
        println!("result: {invalid} action(s) FAILED — bundle not fully valid");
        process::exit(1);
    }
}

// ── record ─────────────────────────────────────────────────────────────────

fn cmd_record(args: &[String]) {
    // tl-agent record <bundle_dir> <action_id> <result_hash> [--verifier <path>]
    if args.len() < 5 {
        eprintln!("usage: tl-agent record <bundle_dir> <action_id> <result_hash> [--verifier <path>]");
        process::exit(1);
    }
    let bundle_dir  = PathBuf::from(&args[2]);
    let action_id   = &args[3];
    let result_hash = &args[4];
    let verifier    = parse_verifier_flag(args);

    let bundle = load_bundle(&bundle_dir, verifier.as_deref());

    match bundle.record_execution(action_id, result_hash) {
        Ok(()) => {
            println!("recorded");
            println!("  action : {action_id}");
            println!("  digest : {result_hash}");
            println!("  log    : {}/execution_log.jsonl", bundle_dir.display());
            process::exit(0);
        }
        Err(e) => {
            eprintln!("record failed: {e}");
            process::exit(1);
        }
    }
}

// ── helpers ────────────────────────────────────────────────────────────────

fn load_bundle(bundle_dir: &PathBuf, verifier_hint: Option<&std::path::Path>) -> AgentBundle {
    if !bundle_dir.exists() {
        eprintln!("tl-agent: bundle directory not found: {}", bundle_dir.display());
        process::exit(1);
    }
    match AgentBundle::load_with_verifier(bundle_dir, verifier_hint) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("tl-agent: failed to load bundle: {e}");
            process::exit(1);
        }
    }
}

fn parse_verifier_flag(args: &[String]) -> Option<PathBuf> {
    args.windows(2)
        .find(|w| w[0] == "--verifier")
        .map(|w| PathBuf::from(&w[1]))
}

fn print_usage() {
    println!("tl-agent — TimeLayer Receipt-Gated Agent CLI");
    println!();
    println!("USAGE:");
    println!("  tl-agent check  <bundle_dir> <action_id> [--verifier <path>]");
    println!("  tl-agent next   <bundle_dir> <action_id> [--verifier <path>]");
    println!("  tl-agent audit  <bundle_dir>             [--verifier <path>]");
    println!("  tl-agent record <bundle_dir> <action_id> <result_hash> [--verifier <path>]");
    println!();
    println!("COMMANDS:");
    println!("  check   Run gate check for one action. Exit 0 = ALLOW, exit 1 = STOP.");
    println!("  next    List actions allowed after <action_id> per topology.");
    println!("  audit   Check every action in the bundle. Exit 0 = all valid.");
    println!("  record  Append execution log entry (does NOT issue a .tlsig).");
    println!();
    println!("OPTIONS:");
    println!("  --verifier <path>   Path to timelayer-verifier binary.");
    println!("                      Auto-resolved from: next to tl-agent, bundle/bin/, PATH.");
}
