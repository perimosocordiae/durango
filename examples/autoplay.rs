use clap::Parser;
use durango::agent;
use durango::game;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value_t = 2)]
    players: usize,
    #[clap(short, long, default_value_t = 100)]
    actions: usize,
}

fn main() {
    let args = Args::parse();
    let mut g = match game::GameState::new(args.players, &mut rand::rng()) {
        Ok(game) => game,
        Err(e) => {
            eprintln!("Error creating game state: {}", e);
            return;
        }
    };
    let ais = (0..args.players)
        .map(|_| agent::create_agent(0))
        .collect::<Vec<_>>();
    for _ in 0..args.actions {
        println!("P{} hand: {:?}", g.curr_player_idx, g.curr_player().hand);
        let act = ais[g.curr_player_idx].choose_action(&g);
        println!(" action: {:?}", &act);
        match g.process_action(&act) {
            Ok(()) => {}
            Err(e) => {
                println!("Error processing action: {}", e);
                break;
            }
        }
    }
}
