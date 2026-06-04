# src/bin/softmax_match.rs

## Role In The System

`src/bin/softmax_match.rs` runs one or more games between two `SoftmaxBot`
players. It is a small command-line app for experimenting with temperatures,
weak solving, starting positions, deterministic seeds, and first-player
alternation.

Related concept notes:

- [notes/concepts/softmax_bots.md](../../concepts/softmax_bots.md)
- [notes/concepts/connect4_rules.md](../../concepts/connect4_rules.md)
- [notes/concepts/solver_scores.md](../../concepts/solver_scores.md)
- [notes/concepts/opening_books.md](../../concepts/opening_books.md)

## Code Walkthrough

### Crate Attribute

```rust
#![forbid(unsafe_code)]
```

The binary forbids unsafe code.

### Imports

```rust
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use exact_connect4::position::{BOARD_SIZE, Position};
use exact_connect4::softmax_bot::{SoftmaxBot, SoftmaxBotConfig};
```

The binary uses `clap` for options, `ExitCode` for process status, and the
library position and bot types for gameplay.

### Options

```rust
#[derive(Debug, Parser)]
#[command(
    name = "softmax_match",
    about = "Run matches between two softmax Connect4 bots"
)]
struct Options {
    #[arg(long, default_value_t = 1)]
    games: usize,

    #[arg(long, default_value_t = 0.0)]
    temperature1: f64,

    #[arg(long, default_value_t = 0.0)]
    temperature2: f64,

    #[arg(long, value_name = "FILE", default_value = exact_connect4::DEFAULT_OPENING_BOOK_PATH)]
    book: PathBuf,

    #[arg(long, default_value = "")]
    start: String,

    #[arg(long)]
    seed: Option<u64>,

    #[arg(long)]
    fixed_first: bool,

    #[arg(long)]
    weak: bool,

    #[arg(long)]
    verbose: bool,
}
```

The options mirror the gameplay knobs: number of games, two temperatures, book
path, starting sequence, optional seed, first-player alternation, weak solving,
and verbose per-game output.

### GameResult

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct GameResult {
    moves: String,
    winner: i32,
    score: i32,
    first_player_bot: i32,
}
```

`GameResult` records the played move sequence, winner, Bot 1's signed score,
and which bot moved first.

### Main Function

```rust
fn main() -> ExitCode {
    let options = Options::parse();
    if let Err(message) = validate_options(&options) {
        eprintln!("{message}");
        return ExitCode::from(1);
    }
```

The binary parses options and validates game count, temperatures, and the
starting position.

```rust
    let bot1_config = SoftmaxBotConfig {
        temperature: options.temperature1,
        seed: options.seed,
        opening_book: Some(options.book.clone()),
        weak: options.weak,
    };
    let bot2_config = SoftmaxBotConfig {
        temperature: options.temperature2,
        seed: options.seed.map(|seed| seed.wrapping_add(1)),
        opening_book: Some(options.book.clone()),
        weak: options.weak,
    };
```

Each bot receives its own temperature and the same book path. When a seed is
provided, Bot 2 uses the next seed value.

```rust
    let mut bot1 = match SoftmaxBot::from_config(bot1_config) {
        Ok(bot) => bot,
        Err(error) => {
            eprintln!("Unable to initialize Bot 1: {error}");
            return ExitCode::from(1);
        }
    };
    let mut bot2 = match SoftmaxBot::from_config(bot2_config) {
        Ok(bot) => bot,
        Err(error) => {
            eprintln!("Unable to initialize Bot 2: {error}");
            return ExitCode::from(1);
        }
    };
```

Book-loading failures are reported as initialization errors.

```rust
    let mut bot1_wins = 0;
    let mut bot2_wins = 0;
    let mut draws = 0;
    let mut total_score = 0;

    for game_index in 0..options.games {
        let result = play_game(&mut bot1, &mut bot2, &options, game_index);

        match result.winner {
            1 => bot1_wins += 1,
            2 => bot2_wins += 1,
            _ => draws += 1,
        }
        total_score += result.score;

        if options.verbose {
            print_game_result(game_index, &result);
        }
    }
```

The main loop plays each game, updates summary counters, and optionally prints
per-game details.

```rust
    print_summary(&options, bot1_wins, bot2_wins, draws, total_score);
    ExitCode::SUCCESS
}
```

After all games, the binary prints the aggregate summary.

### Validation

```rust
fn validate_options(options: &Options) -> Result<(), String> {
    if options.games == 0 {
        return Err("The number of games must be positive.".to_string());
    }
    if !options.temperature1.is_finite() || !options.temperature2.is_finite() {
        return Err("Temperatures must be finite.".to_string());
    }
    if options.temperature1 < 0.0 || options.temperature2 < 0.0 {
        return Err("Temperatures must be non-negative.".to_string());
    }
    Position::from_sequence(&options.start)
        .map_err(|_| format!("Invalid starting position: {:?}", options.start))?;
    Ok(())
}
```

Validation keeps match setup errors out of the gameplay loop. Temperatures must
be finite and non-negative.

### Playing One Game

```rust
fn play_game(
    bot1: &mut SoftmaxBot,
    bot2: &mut SoftmaxBot,
    options: &Options,
    game_index: usize,
) -> GameResult {
    let first_player_bot = if !options.fixed_first && game_index % 2 == 1 {
        2
    } else {
        1
    };
    let mut result = GameResult {
        moves: options.start.clone(),
        winner: 0,
        score: 0,
        first_player_bot,
    };
    let mut position = Position::from_sequence(&options.start).expect("start was validated");
```

Games alternate the first player unless `--fixed-first` is set. The starting
position has already been validated.

```rust
    while position.move_count() < BOARD_SIZE {
        let first_player_to_move = position.move_count().is_multiple_of(2);
        let bot_to_move = if first_player_to_move {
            result.first_player_bot
        } else {
            3 - result.first_player_bot
        };
        let bot = if bot_to_move == 1 {
            &mut *bot1
        } else {
            &mut *bot2
        };
```

The side to move is mapped back to Bot 1 or Bot 2, accounting for first-player
alternation.

```rust
        let selection = bot.select_move(&position);
        let Some(column) = selection.column else {
            break;
        };

        let winning_move = position.is_winning_move(column);
        result.moves.push(char::from(b'1' + column as u8));
```

The bot selects a move. If no move is available, the game is a draw. Otherwise,
the selected 0-based column is appended as a 1-based digit.

```rust
        if winning_move {
            let winning_score = ((BOARD_SIZE + 1 - position.move_count()) as i32) / 2;
            result.winner = bot_to_move;
            result.score = if bot_to_move == 1 {
                winning_score
            } else {
                -winning_score
            };
            break;
        }

        position
            .play_col(column)
            .expect("selected column should be playable");
    }

    result
}
```

Winning moves finish the game immediately and assign Bot 1's signed score.
Non-winning moves are played into the position.

### Output Helpers

```rust
fn print_game_result(game_index: usize, result: &GameResult) {
    print!(
        "Game {}: first player = Bot {}, winner = ",
        game_index + 1,
        result.first_player_bot
    );
    if result.winner == 0 {
        print!("draw");
    } else {
        print!("Bot {}", result.winner);
    }
    println!(
        ", score for Bot 1 = {}, moves = {}",
        result.score, result.moves
    );
}
```

`print_game_result` prints one verbose game line.

```rust
fn print_summary(
    options: &Options,
    bot1_wins: usize,
    bot2_wins: usize,
    draws: usize,
    total_score: i32,
) {
    println!("Bot 1 temperature: {}", options.temperature1);
    println!("Bot 2 temperature: {}", options.temperature2);
    println!("Games played: {}", options.games);
    println!("Bot 1 wins: {bot1_wins}");
    println!("Bot 2 wins: {bot2_wins}");
    println!("Draws: {draws}");
    println!(
        "Average score for Bot 1: {:.3}",
        total_score as f64 / options.games as f64
    );
}
```

`print_summary` prints aggregate match statistics.
