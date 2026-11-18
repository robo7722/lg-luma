use super::*;
use crate::{model::Design, renderer::Draw, state::RETRO_ROWS};

fn retro() -> State {
    let mut state = ready();
    state.preferences.design = Design::Retro;
    state.focus = state.first_content();
    state
}

#[test]
fn design_defaults_to_glass_and_round_trips() {
    let old: Preferences = serde_json::from_str("{}").unwrap();
    assert_eq!(old.design, Design::Glass);
    let preferences = Preferences {
        design: Design::Retro,
        ..Preferences::default()
    };
    let json = serde_json::to_string(&preferences).unwrap();
    assert!(json.contains("\"design\":\"retro\""));
    let loaded: Preferences = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.design, Design::Retro);
}

#[test]
fn general_setting_toggles_the_design_and_saves() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Setting(13);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.design, Design::Retro);
    assert!(
        scene(&state)
            .controls
            .iter()
            .any(|control| control.target == Target::Setting(13))
    );
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.design, Design::Glass);
}

#[test]
fn retro_home_lists_favorites_then_fixed_rows() {
    let state = retro();
    let rows = state.retro_rows();
    let favorites = state.dock_apps().len();
    assert_eq!(rows.len(), favorites + 4);
    assert_eq!(state.focus, rows[0]);
    assert_eq!(
        &rows[favorites..],
        &[
            Target::Nav(1),
            Target::InputsMenu,
            Target::LumaSettings,
            Target::TvSettings
        ]
    );
}

#[test]
fn up_and_down_walk_the_list_and_stop_at_its_ends() {
    let mut state = retro();
    let rows = state.retro_rows();
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, rows[0]);
    state.direction(Direction::Left, &controls(&state));
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, rows[0]);
    for row in rows.iter().skip(1) {
        state.direction(Direction::Down, &controls(&state));
        assert_eq!(&state.focus, row);
    }
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, *rows.last().unwrap());
    assert_eq!(state.page, Page::Home);
}

#[test]
fn only_the_current_page_of_rows_is_drawn_and_clickable() {
    let mut state = retro();
    let rows = state.retro_rows();
    assert!(
        rows.len() > RETRO_ROWS,
        "demo inventory should need two pages"
    );
    let first: Vec<_> = controls(&state).into_iter().map(|c| c.target).collect();
    assert_eq!(first, rows[..RETRO_ROWS]);
    state.focus = rows[RETRO_ROWS].clone();
    let second: Vec<_> = controls(&state).into_iter().map(|c| c.target).collect();
    assert_eq!(second, rows[RETRO_ROWS..]);
}

#[test]
fn digits_open_the_numbered_row() {
    let mut state = retro();
    let rows = state.retro_rows();
    let Target::App(id) = rows[1].clone() else {
        panic!("second row should be a favorite");
    };
    assert_eq!(state.retro_digit(2), vec![Effect::Launch(id)]);
    let mut state = retro();
    let all_apps = rows.iter().position(|row| *row == Target::Nav(1)).unwrap();
    if all_apps < RETRO_ROWS {
        assert!(state.retro_digit(all_apps + 1).is_empty());
        assert_eq!(state.page, Page::Apps);
    }
    let mut state = retro();
    assert!(state.retro_digit(0).is_empty());
    state.preferences.design = Design::Glass;
    assert!(state.retro_digit(1).is_empty());
}

#[test]
fn move_mode_reorders_favorites_with_up_and_down() {
    let mut state = retro();
    let first = state.focus.clone();
    hold_and_move(&mut state);
    assert_eq!(
        state.direction(Direction::Down, &controls(&state)),
        vec![Effect::Save]
    );
    assert_eq!(state.retro_rows()[1], first);
    assert!(
        state
            .direction(Direction::Left, &controls(&state))
            .is_empty()
    );
    assert_eq!(state.retro_rows()[1], first);
    assert_eq!(
        state.direction(Direction::Up, &controls(&state)),
        vec![Effect::Save]
    );
    assert_eq!(state.retro_rows()[0], first);
}

#[test]
fn retro_home_draws_no_glass_and_glass_home_is_unchanged() {
    let state = retro();
    assert!(
        !scene(&state)
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Glass(..) | Draw::Rim(..)))
    );
    let mut glass = retro();
    glass.preferences.design = Design::Glass;
    glass.focus = glass.first_content();
    assert!(
        scene(&glass)
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Glass(..)))
    );
}

#[test]
fn other_pages_keep_their_navigation_in_retro_design() {
    let mut state = retro();
    state.open(2);
    assert!(!state.retro_home());
    assert_eq!(state.focus, Target::Sidebar(0));
}
