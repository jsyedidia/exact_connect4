# Softmax Bots

A softmax bot is a player that asks the exact solver to score legal moves, then
uses those scores to choose a move probabilistically. It sits between two
extremes:

- A perfect player always chooses one of the best-scoring moves.
- A purely random player ignores the solver scores.

The softmax bot still uses the exact solver to understand the position, but it
can deliberately make weaker moves with controlled probability.

## Scores Become Preferences

For a position, the solver can produce one score per column. Illegal columns
use `INVALID_MOVE` and are ignored by the bot. Every remaining move becomes a
`ScoredMove` with:

- a zero-based column,
- the solver score for that move,
- a selection probability.

Higher scores should receive higher probability. The temperature controls how
strongly score differences matter.

## Greedy Mode

When the temperature is zero, or extremely close to zero, the bot uses greedy
selection instead of exponentials.

Greedy selection gives all probability to the best-scoring moves. If one move
is best, it gets probability `1.0`. If several moves tie for best, probability
is split equally among them.

For example, scores:

```text
2, 4, 4, 1
```

produce probabilities:

```text
0.0, 0.5, 0.5, 0.0
```

This is the behavior of a perfect player except that ties are sampled randomly.

## Positive Temperatures

For positive temperatures, the bot uses softmax:

```text
weight(move) = exp((score(move) - best_score) / temperature)
probability(move) = weight(move) / sum(weights)
```

Subtracting `best_score` keeps the exponentials numerically stable. The best
move gets exponent `0`, so its weight is `1`. Worse moves get negative
exponents and weights between `0` and `1`.

Temperature changes how sharp the distribution is:

- Low positive temperature makes the bot nearly greedy.
- Higher temperature gives weaker moves more probability.
- Very high temperature approaches a more uniform distribution over legal
  moves.

The bot includes a fallback to uniform probabilities if all floating-point
weights underflow to zero.

## Weak Mode

The bot can use either exact scores or weak win/draw/loss scores.

Exact scores preserve distance-to-win information. A move that wins quickly
gets a larger score than a move that wins slowly.

Weak scores collapse the result to:

```text
1   win
0   draw
-1  loss
```

Weak mode is useful when the bot should care about result class but not about
how quickly the game ends. It also makes many more moves tie, so greedy weak
play can be less deterministic than greedy exact play.

## Randomness And Seeds

After probabilities are assigned, the bot samples from the distribution with a
random number generator.

An optional seed makes the sequence of sampled moves reproducible. This is
important for tests and for comparing experiments. Without a seed, the bot uses
OS randomness.

When two bots are created for a match from one base seed, the match code gives
the second bot a different seed by adding one to the base seed. That prevents
the two players from sharing the same random stream.

## Implemented In This Repo

Softmax play is implemented in `src/softmax_bot.rs` and used by
`src/bin/softmax_match.rs`.

The main code-level pieces are:

- `SoftmaxBot` owns a `Solver`, a temperature, a weak-mode flag, and a random
  engine.
- `SoftmaxBotConfig` chooses the temperature, optional seed, optional
  opening-book path, and weak mode.
- `SoftmaxBot::evaluate_moves` calls `Solver::analyze`, filters out
  `INVALID_MOVE`, and builds a `Vec<ScoredMove>`.
- `assign_softmax_probabilities` implements greedy behavior and positive
  temperature softmax.
- `SoftmaxBot::select_move` samples the probability distribution with
  `rand::distr::weighted::WeightedIndex`.
- `SoftmaxMoveSelection` returns both the selected move and the complete scored
  move list, so callers can inspect the distribution.
- `src/bin/softmax_match.rs` creates two bots, alternates first player unless
  `--fixed-first` is set, and reports match results.

The matching source notes are
[notes/src/softmax_bot.rs.md](../src/softmax_bot.rs.md) and
[notes/src/bin/softmax_match.rs.md](../src/bin/softmax_match.rs.md).
