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
    assert!(!state.preferences.retro_music);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.design, Design::Retro);
    // Switching to Retro TV turns its background music on.
    assert!(state.preferences.retro_music);
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
    assert!(!state.retro_list());
    assert_eq!(state.focus, Target::Sidebar(0));
}

#[test]
fn background_music_is_off_by_default_and_only_offered_in_retro() {
    assert!(!Preferences::default().retro_music);
    let old: Preferences = serde_json::from_str("{}").unwrap();
    assert!(!old.retro_music);
    let mut state = ready();
    state.open(2);
    let has_row = |state: &State| {
        scene(state)
            .controls
            .iter()
            .any(|c| c.target == Target::Setting(14))
    };
    assert!(!has_row(&state));
    state.preferences.design = Design::Retro;
    assert!(has_row(&state));
    state.focus = Target::Setting(14);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.retro_music);
}

#[test]
fn music_plays_only_on_retro_home_and_stops_for_a_launch() {
    let mut state = retro();
    assert!(!state.retro_music_playing(), "off until enabled");
    state.preferences.retro_music = true;
    assert!(state.retro_music_playing());
    state.launching = true;
    assert!(!state.retro_music_playing());
    state.launching = false;
    // All apps is a retro menu page too: the music keeps playing there.
    state.open(1);
    assert!(state.retro_music_playing());
    state.open(2);
    assert!(!state.retro_music_playing());
    state.open(0);
    state.preferences.design = Design::Glass;
    assert!(!state.retro_music_playing());
}

#[test]
fn retro_design_plays_retro_clicks() {
    use crate::audio::Cue;
    let mut state = retro();
    state.cues.clear();
    state.navigate(Direction::Down, &scene(&state).controls);
    state.navigate(Direction::Up, &scene(&state).controls);
    state.navigate(Direction::Up, &scene(&state).controls);
    assert_eq!(
        state.cues,
        [Cue::RetroFocus, Cue::RetroFocus, Cue::RetroBump]
    );
    state.cues.clear();
    state.tracking_cue();
    assert_eq!(state.cues, [Cue::Tracking]);
    state.preferences.reduced_motion = true;
    state.tracking_cue();
    assert_eq!(state.cues, [Cue::Tracking]);
    let mut glass = ready();
    glass.cues.clear();
    glass.navigate(Direction::Right, &scene(&glass).controls);
    assert_eq!(glass.cues, [Cue::Focus]);
}

#[test]
fn retro_settings_keep_their_controls_but_lose_the_glass() {
    // System: the same rows in both designs (General adds the retro music switch).
    let mut glass = ready();
    glass.open(2);
    glass.section = crate::state::SYSTEM_SECTION;
    let mut retro = ready();
    retro.preferences.design = Design::Retro;
    retro.open(2);
    retro.section = crate::state::SYSTEM_SECTION;
    let glass_scene = scene(&glass);
    let retro_scene = scene(&retro);
    let targets = |scene: &view::Scene| {
        scene
            .controls
            .iter()
            .map(|control| (control.target.clone(), control.rect.x, control.rect.y))
            .collect::<Vec<_>>()
    };
    assert_eq!(targets(&glass_scene), targets(&retro_scene));
    assert!(
        glass_scene
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Glass(..)))
    );
    assert!(
        !retro_scene
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Glass(..) | Draw::Rim(..) | Draw::Shadow(..)))
    );
}

#[test]
fn turning_retro_off_keeps_the_music_choice_but_silences_it() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Setting(13);
    state.activate();
    state.focus = Target::Setting(14);
    state.activate();
    assert!(!state.preferences.retro_music, "switched off by hand");
    state.focus = Target::Setting(13);
    state.activate();
    assert_eq!(state.preferences.design, Design::Glass);
    state.open(0);
    assert!(!state.retro_music_playing());
}

fn retro_apps() -> State {
    let mut state = retro();
    let all_apps = state
        .retro_rows()
        .iter()
        .position(|row| *row == Target::Nav(1))
        .unwrap();
    state.focus = state.retro_rows()[all_apps].clone();
    state.activate();
    assert_eq!(state.page, Page::Apps);
    state
}

#[test]
fn all_apps_opens_as_a_retro_list_of_the_drawer_apps() {
    let state = retro_apps();
    let rows = state.retro_rows();
    let drawer: Vec<_> = state
        .drawer_apps()
        .iter()
        .map(|app| Target::App(app.id.clone()))
        .collect();
    assert_eq!(rows, drawer);
    assert_eq!(state.focus, rows[0], "the list starts on its first app");
    let scene = scene(&state);
    assert!(
        !scene
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Glass(..) | Draw::Rim(..)))
    );
    let targets: Vec<_> = scene.controls.into_iter().map(|c| c.target).collect();
    assert_eq!(targets, rows[..RETRO_ROWS.min(rows.len())]);
}

#[test]
fn all_apps_walks_with_up_and_down_launches_and_goes_back_home() {
    let mut state = retro_apps();
    let rows = state.retro_rows();
    state.direction(Direction::Down, &scene(&state).controls);
    assert_eq!(state.focus, rows[1]);
    state.direction(Direction::Right, &scene(&state).controls);
    assert_eq!(state.focus, rows[1]);
    for _ in 0..RETRO_ROWS {
        state.direction(Direction::Down, &scene(&state).controls);
    }
    assert_eq!(
        state.focus,
        rows[RETRO_ROWS + 1],
        "the next page follows the focus"
    );
    let Target::App(id) = rows[RETRO_ROWS + 1].clone() else {
        panic!("apps only");
    };
    assert_eq!(state.activate(), vec![Effect::Launch(id)]);
    state.launching = false;
    state.back();
    assert_eq!(state.page, Page::Home);
}

#[test]
fn all_apps_digits_open_the_row_on_the_current_page() {
    let mut state = retro_apps();
    let Target::App(id) = state.retro_rows()[2].clone() else {
        panic!("apps only");
    };
    assert_eq!(state.retro_digit(3), vec![Effect::Launch(id)]);
}

#[test]
fn all_apps_move_mode_reorders_with_up_and_down() {
    let mut state = retro_apps();
    let first = state.focus.clone();
    hold_and_move(&mut state);
    assert_eq!(
        state.direction(Direction::Down, &scene(&state).controls),
        vec![Effect::Save]
    );
    assert_eq!(state.retro_rows()[1], first);
    assert_eq!(
        state.preferences.app_sorting,
        crate::model::AppSorting::Custom
    );
}

#[test]
fn tv_faults_move_with_the_clock_and_stop_with_reduced_motion() {
    let assets = view::Assets {
        root: PathBuf::new(),
        version: "test".into(),
        metrics: BTreeMap::new(),
        artwork: crate::app_artwork::Service::new(false),
    };
    let at = |state: &State, ms: i64| {
        let now = time::OffsetDateTime::UNIX_EPOCH + time::Duration::milliseconds(ms);
        view::scene(state, &assets, now)
            .draw
            .iter()
            .filter_map(|draw| match draw {
                Draw::Shape(rect, ..) if rect.w >= 1920. && rect.h < 30. => Some(rect.y),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let mut state = retro();
    assert!(state.retro_faults());
    // Hum bar strips are full-width and change place as the clock moves on.
    assert_ne!(at(&state, 1_000), at(&state, 2_000));
    state.preferences.reduced_motion = true;
    assert!(!state.retro_faults());
    assert_eq!(at(&state, 1_000), at(&state, 2_000));
    state.preferences.reduced_motion = false;
    state.preferences.design = Design::Glass;
    assert!(!state.retro_faults());
}

#[test]
fn switching_the_design_plays_the_robos_boot() {
    use crate::audio::Cue;
    let mut state = ready();
    state.open(2);
    state.cues.clear();
    state.focus = Target::Setting(13);
    state.activate();
    let (_, design) = state.design_boot.expect("boot after switching to Retro");
    assert_eq!(design, Design::Retro);
    assert!(state.design_boot_at().is_some());
    assert!(state.animating(), "the boot animates");
    assert!(state.cues.contains(&Cue::Boot));
    // Mid-boot, after the CRT power-on, the RobOS mark is on screen.
    state.design_boot = Some((
        std::time::Instant::now() - std::time::Duration::from_millis(1000),
        Design::Retro,
    ));
    let boot = scene(&state);
    assert!(boot.draw.iter().any(|draw| matches!(draw,
        Draw::TintedImage(path, ..) if path.ends_with("robos-mark.lvec"))));
    // Back to Liquid Glass boots too; with Reduce motion only the sound plays.
    state.preferences.reduced_motion = true;
    state.design_boot = None;
    state.cues.clear();
    state.activate();
    assert_eq!(state.preferences.design, Design::Glass);
    assert!(state.design_boot.is_none());
    assert!(state.cues.contains(&Cue::Boot));
}

#[test]
fn the_boot_keeps_redrawing_and_ends_on_the_page() {
    let mut state = ready();
    state.design_boot = Some((std::time::Instant::now(), Design::Glass));
    state.dirty = false;
    state.tick();
    assert!(state.dirty, "every boot frame is drawn");
    state.design_boot = Some((
        std::time::Instant::now() - std::time::Duration::from_secs(3),
        Design::Glass,
    ));
    state.dirty = false;
    state.tick();
    assert!(state.design_boot.is_none());
    assert!(
        state.dirty,
        "one more frame shows the page without the overlay"
    );
}
