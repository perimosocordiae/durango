use clap::Parser;
use durango::agent;
use durango::game;
use durango::game::ActionOutcome;
use rand::{Rng, SeedableRng};

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
    #[clap(long, value_parser, value_delimiter = ',', default_value = "0,1")]
    ai_levels: Vec<usize>,
    #[clap(long)]
    seed: Option<u64>,
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

fn run_game(args: &Args, rng: &mut impl Rng) -> Option<RunInfo> {
    let mut g = match game::GameState::new(args.players, &args.preset, rng) {
        Ok(game) => game,
        Err(e) => {
            eprintln!("Error creating game state: {}", e);
            return None;
        }
    };
    let ais = (0..args.players)
        .map(|i| agent::create_agent(args.ai_levels[i % args.ai_levels.len()]))
        .collect::<Vec<_>>();
    for a in 0..args.actions {
        if !args.quiet {
            println!("{}", g.curr_player().debug_str(g.curr_player_idx));
        }
        let is_user = args.interactive && g.curr_player_idx == 0;
        let act = if is_user {
            interactive_action(&g)
        } else {
            ais[g.curr_player_idx].choose_action(&g, rng)
        };
        if !args.quiet {
            println!(" action: {:?}", &act);
        }
        match g.process_action(&act, rng) {
            Ok(ActionOutcome::GameOver) => {
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
            Ok(_) => {}
            Err(e) => {
                println!(
                    "Error processing {act:?} for player {}:\n{e}",
                    g.curr_player_idx
                );
                if !is_user {
                    return None;
                }
            }
        }
    }
    println!("Ran out of actions on {}", args.preset);
    println!("Player states:");
    for i in 0..args.players {
        println!(" - {}", g.players[i].debug_str(i));
    }
    println!("Scores: {:?}\n", g.player_scores());
    None
}

const ALL_PRESETS: &[&str] = &[
    "first", "easy1", "easy2", "medium1", "medium2", "hard1", "hard2",
];

struct Stats {
    count: usize,
    min: usize,
    max: usize,
    sum: usize,
    sum_sq: usize,
}
impl Stats {
    fn new() -> Self {
        Stats {
            count: 0,
            min: usize::MAX,
            max: usize::MIN,
            sum: 0,
            sum_sq: 0,
        }
    }

    fn add(&mut self, value: usize) {
        self.count += 1;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        self.sum += value;
        self.sum_sq += value * value;
    }

    fn report(&self, name: &str) {
        let mean = self.sum as f64 / self.count as f64;
        let variance = (self.sum_sq as f64 / self.count as f64) - (mean * mean);
        println!(
            "{name}: min={}, max={}, mean={mean:.2}, stddev={:.2}",
            self.min,
            self.max,
            variance.sqrt()
        );
    }
}

fn main() {
    let mut args = Args::parse();
    let all_presets = args.preset == "all";
    let mut time_stats = Stats::new();
    let mut round_stats = Stats::new();
    let mut action_stats = Stats::new();
    let mut win_counts = vec![0; args.players];
    let mut rng = if let Some(seed) = args.seed {
        rand::rngs::StdRng::seed_from_u64(seed)
    } else {
        rand::rngs::StdRng::from_rng(&mut rand::rng())
    };
    for i in 0..args.repeats {
        if all_presets {
            args.preset = ALL_PRESETS[i % ALL_PRESETS.len()].to_string();
        }
        let start_time = std::time::Instant::now();
        if let Some(info) = run_game(&args, &mut rng) {
            round_stats.add(info.rounds);
            action_stats.add(info.actions);
            win_counts[info.winner] += 1;
        }
        let elapsed = start_time.elapsed();
        time_stats.add(elapsed.as_millis() as usize);
    }
    println!(
        "{} out of {} games were successful",
        round_stats.count, args.repeats
    );
    time_stats.report("Time ms");
    if round_stats.count > 0 {
        round_stats.report("Rounds ");
        action_stats.report("Actions");
    }
    for (i, count) in win_counts.iter().enumerate() {
        println!("Player {i}: {count} wins");
    }
}
