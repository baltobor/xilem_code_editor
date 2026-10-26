// Copyright 2026 the xilem_code_editor developers
// SPDX-License-Identifier: Apache-2.0

//! Auto-trigger policy for completion requests while typing.

mod completes_trigger_sequence;
mod identifier_run_length;
mod is_identifier_char;
mod is_noisy_trigger_char;
mod is_trigger_char;
mod min_prefix;
mod should_auto_trigger;

#[cfg(test)]
mod tests;

pub use completes_trigger_sequence::completes_trigger_sequence;
pub use identifier_run_length::identifier_run_length_ending_at;
pub use is_identifier_char::is_identifier_char;
pub use is_noisy_trigger_char::is_noisy_trigger_char;
pub use is_trigger_char::is_trigger_char;
pub use min_prefix::AUTO_TRIGGER_MIN_PREFIX;
pub use should_auto_trigger::should_auto_trigger;
