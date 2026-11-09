use super::*;

fn fixed(value: u32) -> impl FnMut() -> u32 {
    move || value
}

#[test]
fn single_names_resolve_like_before() {
    let mut roll = fixed(0);
    assert_eq!(plan("fade", &mut roll), Plan::Single(Part::Fade));
    assert_eq!(plan("", &mut roll), Plan::Single(Part::Fade));
    assert_eq!(plan("no-such-thing", &mut roll), Plan::Single(Part::Fade));
    let burn = effect_index("burn").unwrap();
    assert_eq!(plan("burn", &mut roll), Plan::Single(Part::Effect(burn)));
    let galaxy = sand_style_index("sand-galaxy").unwrap();
    assert_eq!(plan("sand-galaxy", &mut roll), Plan::Single(Part::Sand(galaxy)));
}

#[test]
fn random_covers_sand_and_effects() {
    let total = (SAND_STYLES.len() + EFFECTS.len()) as u32;
    let mut seen_sand = false;
    let mut seen_effect = false;
    for value in 0..total {
        match plan("random", &mut fixed(value)) {
            Plan::Single(Part::Sand(_)) => seen_sand = true,
            Plan::Single(Part::Effect(_)) => seen_effect = true,
            other => panic!("random gave {other:?}"),
        }
    }
    assert!(seen_sand && seen_effect);
}

#[test]
fn combos_parse_every_mode() {
    let burn = effect_index("burn").unwrap();
    let ripple = effect_index("ripple").unwrap();
    for (key, mode) in COMBO_MODES {
        let name = format!("combo:{key}:burn:ripple");
        let expected = Combo { mode, first: Part::Effect(burn), second: Part::Effect(ripple) };
        assert_eq!(plan(&name, &mut fixed(0)), Plan::Combo(expected), "{name}");
        assert_eq!(describe(Plan::Combo(expected)), name);
    }
    assert_eq!(
        plan("combo:then:fade:sand-galaxy", &mut fixed(0)),
        Plan::Combo(Combo {
            mode: Mode::Then,
            first: Part::Fade,
            second: Part::Sand(sand_style_index("sand-galaxy").unwrap()),
        })
    );
}

#[test]
fn broken_combos_fade() {
    for name in ["combo:", "combo:then", "combo:then:burn", "combo:wobble:burn:ripple"] {
        assert_eq!(plan(name, &mut fixed(0)), Plan::Single(Part::Fade), "{name}");
    }
}

#[test]
fn random_mix_is_a_combo() {
    for value in 0..40 {
        assert!(matches!(plan(RANDOM_MIX, &mut fixed(value)), Plan::Combo(_)));
    }
}

#[test]
fn shader_ids_are_distinct() {
    let mut ids: Vec<(i32, i32)> =
        COMBO_MODES.iter().map(|(_, mode)| (mode.shader_id(), mode.split_id())).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), COMBO_MODES.len());
}
