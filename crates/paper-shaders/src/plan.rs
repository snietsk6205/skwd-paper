//! Custom build (M3): what a transition name asks for.
//!
//! A name is one transition (`"burn"`, `"sand-galaxy"`, `"fade"`,
//! `"random"`) or two combined: `combo:<mode>:<first>:<second>`, for
//! example `combo:then:burn:ripple` or `combo:split-circle:iris:random`.
//! `random-mix` picks two random transitions and a random way to combine
//! them. Every renderer path (videos, images, scenes, previews) turns its
//! `--shader` value into a [`Plan`] with [`plan`].

use crate::{EFFECTS, SAND_STYLES, effect_index, sand_style_index};

/// One transition: the plain cross-fade, a sand style or an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Fade,
    Sand(i32),
    Effect(usize),
}

/// Shapes of the split-screen combination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Split {
    LeftRight,
    TopBottom,
    Diagonal,
    Circle,
    Checker,
}

/// How the two transitions of a combination work together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The first runs to its half-way point, that frame is frozen, the second
    /// finishes from it.
    Then,
    /// Both run at once; the picture turns from the first into the second.
    Blend,
    /// The first on one part of the screen, the second on the other.
    Split(Split),
    /// The first decides where the change happens, the second how it looks.
    Mask,
    /// A soft line moves across: the first ahead of it, the second behind it.
    Sweep,
    /// Both at once, the brighter of the two wins.
    Light,
    /// Both at once, the darker of the two wins.
    Dark,
}

impl Mode {
    /// Id passed to the compose shader in `u_style` (`Then` has no compose pass).
    pub const fn shader_id(self) -> i32 {
        match self {
            Mode::Then => 0,
            Mode::Blend => 1,
            Mode::Split(_) => 2,
            Mode::Mask => 3,
            Mode::Sweep => 4,
            Mode::Light => 5,
            Mode::Dark => 6,
        }
    }

    /// Split shape id for the compose shader (`u_pid_base`).
    pub const fn split_id(self) -> i32 {
        match self {
            Mode::Split(Split::LeftRight) => 0,
            Mode::Split(Split::TopBottom) => 1,
            Mode::Split(Split::Diagonal) => 2,
            Mode::Split(Split::Circle) => 3,
            Mode::Split(Split::Checker) => 4,
            _ => 0,
        }
    }
}

/// The ways to combine two transitions, by their name in a combo string.
pub const COMBO_MODES: [(&str, Mode); 11] = [
    ("then", Mode::Then),
    ("blend", Mode::Blend),
    ("split-lr", Mode::Split(Split::LeftRight)),
    ("split-tb", Mode::Split(Split::TopBottom)),
    ("split-diagonal", Mode::Split(Split::Diagonal)),
    ("split-circle", Mode::Split(Split::Circle)),
    ("split-checker", Mode::Split(Split::Checker)),
    ("mask", Mode::Mask),
    ("sweep", Mode::Sweep),
    ("light", Mode::Light),
    ("dark", Mode::Dark),
];

/// Prefix of a combination name.
pub const COMBO_PREFIX: &str = "combo:";
/// Two random transitions, combined in a random way.
pub const RANDOM_MIX: &str = "random-mix";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Combo {
    pub mode: Mode,
    pub first: Part,
    pub second: Part,
}

/// What one transition draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Single(Part),
    Combo(Combo),
}

/// Effect index used when a combination needs the plain fade as a pass.
pub fn crossfade_effect() -> usize {
    effect_index("crossfade").unwrap_or(0)
}

/// Resolves one transition name; `"random"` uses `roll`.
pub fn part(name: &str, roll: &mut dyn FnMut() -> u32) -> Part {
    let name = if name == "random" { random_name(roll) } else { name };
    if let Some(style) = sand_style_index(name) {
        return Part::Sand(style);
    }
    effect_index(name).map_or(Part::Fade, Part::Effect)
}

/// A random transition name (sand or effect), like the official "random".
pub fn random_name(roll: &mut dyn FnMut() -> u32) -> &'static str {
    let choices = SAND_STYLES.len() + EFFECTS.len();
    let index = roll() as usize % choices.max(1);
    if index < SAND_STYLES.len() {
        SAND_STYLES[index]
    } else {
        EFFECTS[index - SAND_STYLES.len()].0
    }
}

/// Parses a combination name (`combo:<mode>:<first>:<second>`).
pub fn combo(name: &str, roll: &mut dyn FnMut() -> u32) -> Option<Combo> {
    let rest = name.strip_prefix(COMBO_PREFIX)?;
    let mut fields = rest.splitn(3, ':');
    let mode_name = fields.next()?;
    let first = fields.next()?;
    let second = fields.next()?;
    let mode = COMBO_MODES.iter().find(|(key, _)| *key == mode_name).map(|(_, mode)| *mode)?;
    Some(Combo { mode, first: part(first, roll), second: part(second, roll) })
}

/// Everything a `--shader` value can ask for. Unknown names fade.
pub fn plan(name: &str, roll: &mut dyn FnMut() -> u32) -> Plan {
    if name == RANDOM_MIX {
        let mode = COMBO_MODES[roll() as usize % COMBO_MODES.len()].1;
        let first = part("random", roll);
        let second = part("random", roll);
        return Plan::Combo(Combo { mode, first, second });
    }
    if name.starts_with(COMBO_PREFIX) {
        return combo(name, roll).map_or(Plan::Single(Part::Fade), Plan::Combo);
    }
    Plan::Single(part(name, roll))
}

/// The transition name of a part, for logs.
pub fn part_name(part: Part) -> &'static str {
    match part {
        Part::Fade => "fade",
        Part::Effect(index) => EFFECTS.get(index).map_or("fade", |(name, _)| name),
        Part::Sand(style) => crate::SAND_STYLES
            .iter()
            .copied()
            .find(|name| sand_style_index(name) == Some(style))
            .unwrap_or("sand"),
    }
}

/// A short description of a plan, for logs.
pub fn describe(plan: Plan) -> String {
    match plan {
        Plan::Single(part) => part_name(part).to_string(),
        Plan::Combo(combo) => {
            let mode = COMBO_MODES
                .iter()
                .find(|(_, mode)| *mode == combo.mode)
                .map_or("?", |(key, _)| key);
            format!("{COMBO_PREFIX}{mode}:{}:{}", part_name(combo.first), part_name(combo.second))
        }
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
