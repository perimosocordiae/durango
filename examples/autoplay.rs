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
}

fn interactive_action(g: &game::GameState) -> game::PlayerAction {
    use std::io::{self, Write};
    loop {
        for (idx, card) in g.curr_player().hand.iter().enumerate() {
            println!("  Card {}: {:?}", idx, card);
        }
        for (dir, pos, node) in g.map.neighbors_of(g.curr_player().position) {
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

fn main() {
    let args = Args::parse();
    let mut g = match game::GameState::new(
        args.players,
        &args.preset,
        &mut rand::rng(),
    ) {
        Ok(game) => game,
        Err(e) => {
            eprintln!("Error creating game state: {}", e);
            return;
        }
    };
    let ais = (0..args.players)
        .map(|i| agent::create_agent(i))
        .collect::<Vec<_>>();
    for _ in 0..args.actions {
        println!("{}", g.curr_player().debug_str(g.curr_player_idx));
        let is_user = args.interactive && g.curr_player_idx == 0;
        let act = if is_user {
            interactive_action(&g)
        } else {
            ais[g.curr_player_idx].choose_action(&g)
        };
        println!(" action: {:?}", &act);
        match g.process_action(&act) {
            Ok(true) => {
                println!("Game over! Winners: {:?}", g.players_at_finish());
                break;
            }
            Ok(false) => {}
            Err(e) => {
                eprintln!("Error processing action: {}", e);
                if !is_user {
                    break;
                }
            }
        }
    }
}
