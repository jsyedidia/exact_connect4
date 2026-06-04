# src/softmax_bot.rs

## Role In The System

`src/softmax_bot.rs` defines a gameplay layer on top of the exact solver. It
asks the solver to score legal moves, converts those scores into probabilities,
and samples a move with a random number generator.

The exact solver remains independent from randomness. This file depends on the
solver, but the solver does not depend on this file.

Related concept notes:

- [notes/concepts/softmax_bots.md](../concepts/softmax_bots.md)
- [notes/concepts/solver_scores.md](../concepts/solver_scores.md)
- [notes/concepts/opening_books.md](../concepts/opening_books.md)

## Code Walkthrough

### Imports

```rust
use std::path::PathBuf;

use rand::SeedableRng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::rngs::StdRng;

use crate::opening_book::OpeningBookError;
use crate::position::{Position, WIDTH};
use crate::solver::{INVALID_MOVE, Solver};
```

The module uses `PathBuf` for optional book paths, `rand` for deterministic and
non-deterministic move sampling, and the existing solver types for exact move
scores.

### Greedy Threshold

```rust
const GREEDY_TEMPERATURE_THRESHOLD: f64 = 1e-12;
```

Temperatures at or below this threshold use greedy selection instead of softmax.

### ScoredMove

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredMove {
    pub column: usize,
    pub score: i32,
    pub probability: f64,
}
```

`ScoredMove` stores a legal column, its solver score, and the probability
assigned by the current temperature.

### SoftmaxMoveSelection

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct SoftmaxMoveSelection {
    pub column: Option<usize>,
    pub score: i32,
    pub moves: Vec<ScoredMove>,
}
```

The selected column is optional because a full board has no legal move. The
full scored move list is kept so callers can inspect the probabilities.

### SoftmaxBotConfig

```rust
#[derive(Debug, Clone, Default)]
pub struct SoftmaxBotConfig {
    pub temperature: f64,
    pub seed: Option<u64>,
    pub opening_book: Option<PathBuf>,
    pub weak: bool,
}
```

`temperature` controls greedy versus softmax behavior. `seed` makes sampling
reproducible when present. `opening_book` chooses between an explicit book path
and the embedded default. `weak` chooses exact scores or win/draw/loss scores.

### SoftmaxBot

```rust
#[derive(Debug, Clone)]
pub struct SoftmaxBot {
    solver: Solver,
    temperature: f64,
    weak: bool,
    random_engine: StdRng,
}
```

The bot owns a solver, the temperature setting, the weak-solver flag, and its
random engine.

### Constructors

```rust
impl SoftmaxBot {
    pub fn new() -> Self {
        Self::from_config(SoftmaxBotConfig::default())
            .expect("embedded default opening book should load")
    }

    pub fn from_config(config: SoftmaxBotConfig) -> Result<Self, OpeningBookError> {
        let solver = if let Some(opening_book) = &config.opening_book {
            let mut solver = Solver::without_book();
            solver.load_book_from_file(opening_book)?;
            solver
        } else {
            Solver::new()
        };

        Ok(Self {
            solver,
            temperature: config.temperature,
            weak: config.weak,
            random_engine: make_random_engine(config.seed),
        })
    }
```

`new` uses the default configuration. `from_config` either loads a requested
book file or uses the embedded default book, then creates the random engine.

### Evaluating Moves

```rust
    pub fn evaluate_moves(&mut self, position: &Position) -> Vec<ScoredMove> {
        let scores = self.solver.analyze(position, self.weak);
        let mut moves = Vec::with_capacity(WIDTH);

        for (column, score) in scores.into_iter().enumerate() {
            if score == INVALID_MOVE {
                continue;
            }
            moves.push(ScoredMove {
                column,
                score,
                probability: 0.0,
            });
        }

        assign_softmax_probabilities(&mut moves, self.temperature);
        moves
    }
```

`evaluate_moves` calls `Solver::analyze`, filters out illegal columns, and then
assigns probabilities to the remaining legal moves.

### Selecting A Move

```rust
    pub fn select_move(&mut self, position: &Position) -> SoftmaxMoveSelection {
        let moves = self.evaluate_moves(position);
        if moves.is_empty() {
            return SoftmaxMoveSelection {
                column: None,
                score: INVALID_MOVE,
                moves,
            };
        }

        let distribution =
            WeightedIndex::new(moves.iter().map(|scored_move| scored_move.probability))
                .expect("probabilities should contain at least one positive value");
        let selected_index = distribution.sample(&mut self.random_engine);
        let selected = &moves[selected_index];

        SoftmaxMoveSelection {
            column: Some(selected.column),
            score: selected.score,
            moves,
        }
    }
```

An empty legal-move list produces no selected column. Otherwise, `WeightedIndex`
samples an index using the assigned probabilities.

### Temperature Getter

```rust
    #[must_use]
    pub const fn temperature(&self) -> f64 {
        self.temperature
    }
}
```

The getter exposes the configured temperature.

### Default

```rust
impl Default for SoftmaxBot {
    fn default() -> Self {
        Self::new()
    }
}
```

`Default` delegates to `new`.

### Probability Assignment

```rust
pub fn assign_softmax_probabilities(moves: &mut [ScoredMove], temperature: f64) {
    if moves.is_empty() {
        return;
    }
```

The function is public so probability behavior can be tested without invoking
the solver.

```rust
    if temperature <= GREEDY_TEMPERATURE_THRESHOLD {
        let best_score = moves
            .iter()
            .map(|scored_move| scored_move.score)
            .max()
            .expect("moves is not empty");
        let best_count = moves
            .iter()
            .filter(|scored_move| scored_move.score == best_score)
            .count();
        let best_probability = 1.0 / best_count as f64;

        for scored_move in moves {
            scored_move.probability = if scored_move.score == best_score {
                best_probability
            } else {
                0.0
            };
        }
        return;
    }
```

Greedy mode splits probability evenly among the best-scoring moves and assigns
zero probability to the rest.

```rust
    let max_score = moves
        .iter()
        .map(|scored_move| scored_move.score)
        .max()
        .expect("moves is not empty");
    let weights = moves
        .iter()
        .map(|scored_move| ((scored_move.score - max_score) as f64 / temperature).exp())
        .collect::<Vec<_>>();
    let total_weight = weights.iter().sum::<f64>();
```

Positive-temperature mode subtracts the maximum score before exponentiating.
That keeps the softmax numerically stable.

```rust
    if total_weight <= f64::MIN_POSITIVE {
        let uniform_probability = 1.0 / moves.len() as f64;
        for scored_move in moves {
            scored_move.probability = uniform_probability;
        }
        return;
    }

    for (scored_move, weight) in moves.iter_mut().zip(weights) {
        scored_move.probability = weight / total_weight;
    }
}
```

If all weights underflow, the function falls back to uniform probabilities.
Otherwise, each probability is its weight divided by the total.

### Random Engine

```rust
fn make_random_engine(seed: Option<u64>) -> StdRng {
    match seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_os_rng(),
    }
}
```

A provided seed creates deterministic play. Without a seed, the engine is
seeded from the operating system.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::{ScoredMove, SoftmaxBot, SoftmaxBotConfig, assign_softmax_probabilities};
    use crate::position::Position;

    #[test]
    fn greedy_probabilities_split_between_best_moves() {
        let mut moves = vec![
            scored_move(0, 2),
            scored_move(1, 4),
            scored_move(2, 4),
            scored_move(3, 1),
        ];

        assign_softmax_probabilities(&mut moves, 0.0);

        assert_eq!(moves[0].probability, 0.0);
        assert_eq!(moves[1].probability, 0.5);
        assert_eq!(moves[2].probability, 0.5);
        assert_eq!(moves[3].probability, 0.0);
    }

    #[test]
    fn positive_temperature_probabilities_sum_to_one() {
        let mut moves = vec![scored_move(0, -1), scored_move(1, 0), scored_move(2, 2)];

        assign_softmax_probabilities(&mut moves, 1.5);

        let total = moves
            .iter()
            .map(|scored_move| scored_move.probability)
            .sum::<f64>();
        assert!((total - 1.0).abs() < 1e-12);
        assert!(moves[2].probability > moves[1].probability);
        assert!(moves[1].probability > moves[0].probability);
    }

    #[test]
    fn seeded_selection_is_reproducible() {
        let position = Position::from_sequence("32164625").unwrap();
        let config = SoftmaxBotConfig {
            temperature: 1.0,
            seed: Some(9),
            opening_book: None,
            weak: true,
        };
        let mut bot_a = SoftmaxBot::from_config(config.clone()).unwrap();
        let mut bot_b = SoftmaxBot::from_config(config).unwrap();

        let selected_a = (0..4)
            .map(|_| bot_a.select_move(&position).column)
            .collect::<Vec<_>>();
        let selected_b = (0..4)
            .map(|_| bot_b.select_move(&position).column)
            .collect::<Vec<_>>();

        assert_eq!(selected_a, selected_b);
    }

    fn scored_move(column: usize, score: i32) -> ScoredMove {
        ScoredMove {
            column,
            score,
            probability: 0.0,
        }
    }
}
```

The tests cover greedy tie splitting, positive-temperature normalization, and
seeded reproducibility.
