// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use rand::SeedableRng;
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::rngs::StdRng;

use crate::opening_book::OpeningBookError;
use crate::position::{Position, WIDTH};
use crate::solver::{INVALID_MOVE, Solver};

const GREEDY_TEMPERATURE_THRESHOLD: f64 = 1e-12;

/// A legal move with its solver score and selection probability.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredMove {
    /// Zero-based column index.
    pub column: usize,
    /// Score from the point of view of the side to move.
    pub score: i32,
    /// Probability assigned by greedy or softmax selection.
    pub probability: f64,
}

/// Result returned by `SoftmaxBot::select_move`.
#[derive(Debug, Clone, PartialEq)]
pub struct SoftmaxMoveSelection {
    /// Selected zero-based column, or `None` when no move is available.
    pub column: Option<usize>,
    /// Score of the selected move, or `INVALID_MOVE` when no move is available.
    pub score: i32,
    /// All evaluated legal moves with probabilities.
    pub moves: Vec<ScoredMove>,
}

/// Configuration for `SoftmaxBot`.
#[derive(Debug, Clone, Default)]
pub struct SoftmaxBotConfig {
    /// Softmax temperature. Values near zero use greedy selection.
    pub temperature: f64,
    /// Optional deterministic RNG seed.
    pub seed: Option<u64>,
    /// Optional opening-book path. `None` uses the embedded default book.
    pub opening_book: Option<PathBuf>,
    /// Whether to use weak win/draw/loss solving.
    pub weak: bool,
}

/// Bot that scores legal moves with the exact solver and samples by softmax.
#[derive(Debug, Clone)]
pub struct SoftmaxBot {
    solver: Solver,
    temperature: f64,
    weak: bool,
    random_engine: StdRng,
}

impl SoftmaxBot {
    /// Creates a bot with default configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// let bot = exact_connect4::SoftmaxBot::new();
    ///
    /// assert_eq!(bot.temperature(), 0.0);
    /// ```
    pub fn new() -> Self {
        Self::from_config(SoftmaxBotConfig::default())
            .expect("embedded default opening book should load")
    }

    /// Creates a bot from explicit configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// let bot = exact_connect4::SoftmaxBot::from_config(exact_connect4::SoftmaxBotConfig {
    ///     temperature: 1.0,
    ///     seed: Some(7),
    ///     opening_book: None,
    ///     weak: true,
    /// })?;
    ///
    /// assert_eq!(bot.temperature(), 1.0);
    /// # Ok::<(), exact_connect4::OpeningBookError>(())
    /// ```
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

    /// Scores every legal move and assigns selection probabilities.
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

    /// Selects one legal move according to the configured temperature.
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

    /// Returns the configured temperature.
    #[must_use]
    pub const fn temperature(&self) -> f64 {
        self.temperature
    }
}

impl Default for SoftmaxBot {
    fn default() -> Self {
        Self::new()
    }
}

/// Assigns greedy or softmax probabilities to an already scored move list.
///
/// # Examples
///
/// ```
/// let mut moves = vec![
///     exact_connect4::ScoredMove { column: 0, score: 2, probability: 0.0 },
///     exact_connect4::ScoredMove { column: 1, score: 4, probability: 0.0 },
///     exact_connect4::ScoredMove { column: 2, score: 4, probability: 0.0 },
/// ];
///
/// exact_connect4::softmax_bot::assign_softmax_probabilities(&mut moves, 0.0);
///
/// assert_eq!(moves[0].probability, 0.0);
/// assert_eq!(moves[1].probability, 0.5);
/// assert_eq!(moves[2].probability, 0.5);
/// ```
pub fn assign_softmax_probabilities(moves: &mut [ScoredMove], temperature: f64) {
    if moves.is_empty() {
        return;
    }

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

fn make_random_engine(seed: Option<u64>) -> StdRng {
    match seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_os_rng(),
    }
}

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
