use durango::agent;
use durango::game;

fn main() {
    let num_players = 2;
    let mut g = game::GameState::new(num_players, &mut rand::rng());
    let ais = [agent::create_agent(0), agent::create_agent(0)];
    for _ in 0..100 {
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
