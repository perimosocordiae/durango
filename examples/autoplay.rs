use clap::Parser;
use durango::agent;
use durango::game;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = 2)]
    players: usize,
    #[clap(long, default_value = "easy1")]
    preset: String,
    #[clap(short, long, default_value_t = 100)]
    actions: usize,
    #[clap(short, long)]
    interactive: bool,
    #[clap(short, long)]
    quiet: bool,
    #[clap(long, default_value_t = 1)]
    repeats: usize,
}

fn interactive_action(g: &game::GameState) -> game::PlayerAction {
    use std::io::{self, Write};
    loop {
        for (idx, card) in g.curr_player().hand.iter().enumerate() {
            println!("  Card {}: {:?}", idx, card);
        }
        for (dir, pos, node) in g.neighbors_of(g.curr_player().position) {
            println!("  Move to {:?} at {:?} via {:?}", node.terrain, pos, dir);
        }
        print!("Enter action: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.is_empty() {
            // EOF from Ctrl-D
            println!("\nExiting.");
            std::process::exit(0);
        }
        match serde_json::from_str(&input) {
            Ok(act) => return act,
            Err(e) => println!("Invalid action: {}", e),
        }
    }
}

struct RunInfo {
    rounds: usize,
    actions: usize,
    winner: usize,
}

fn run_game(args: &Args) -> Option<RunInfo> {
    let mut g = match game::GameState::new(
        args.players,
        &args.preset,
        &mut rand::rng(),
    ) {
        Ok(game) => game,
        Err(e) => {
            eprintln!("Error creating game state: {}", e);
            return None;
        }
    };
    let ais = (0..args.players)
        .map(|i| agent::create_agent(i))
        .collect::<Vec<_>>();
    for a in 0..args.actions {
        if !args.quiet {
            println!("{}", g.curr_player().debug_str(g.curr_player_idx));
        }
        let is_user = args.interactive && g.curr_player_idx == 0;
        let act = if is_user {
            interactive_action(&g)
        } else {
            ais[g.curr_player_idx].choose_action(&g)
        };
        if !args.quiet {
            println!(" action: {:?}", &act);
        }
        match g.process_action(&act) {
            Ok(true) => {
                let finishers = g.players_at_finish();
                let rounds = g.round_idx;
                if !args.quiet {
                    println!(
                        "Game over: {rounds} rounds, {a} actions, finished={finishers:?}",
                    );
                }
                return Some(RunInfo {
                    rounds,
                    actions: a,
                    winner: finishers[0],
                });
            }
            Ok(false) => {}
            Err(e) => {
                println!("Error processing action: {}", e);
                if !is_user {
                    return None;
                }
            }
        }
    }
    println!("Ran out of actions");
    println!("Player states:");
    for i in 0..args.players {
        println!("{}", g.players[i].debug_str(i));
    }
    None
}

const ALL_PRESETS: &[&str] = &[
    "first", "easy1", "easy2", "medium1", "medium2", "hard1", "hard2",
];

fn main() {
    let mut args = Args::parse();
    let all_presets = args.preset == "all";
    let mut num_success = 0;
    let mut sum_rounds = 0;
    let mut sum_actions = 0;
    let mut win_counts = vec![0; args.players];
    for i in 0..args.repeats {
        if all_presets {
            args.preset = ALL_PRESETS[i % ALL_PRESETS.len()].to_string();
        }
        if let Some(info) = run_game(&args) {
            num_success += 1;
            sum_rounds += info.rounds;
            sum_actions += info.actions;
            win_counts[info.winner] += 1;
        }
    }
    println!(
        "{} out of {} games were successful",
        num_success, args.repeats
    );
    let denom = num_success.max(1) as f64;
    println!(
        "Average rounds: {:.1}, actions: {:.1}",
        sum_rounds as f64 / denom,
        sum_actions as f64 / denom
    );
    for (i, count) in win_counts.iter().enumerate() {
        println!("Player {i}: {count} wins");
    }
}
