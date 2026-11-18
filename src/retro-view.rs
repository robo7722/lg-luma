//! Retro TV design: Home as a stylised 1990s cable-TV info channel. Framed blue panels, bold
//! yellow headings with a heavy black outline, white labels with yellow values, the selected
//! app on a little broadcast monitor, weather only in the bottom ticker, and faint scanlines. Everything is static while idle except the half-second cursor blink and
//! the ticker line, which changes with a hard cut every few seconds.
use super::{Assets, FACE_HEIGHT, Scene, c, text};
use crate::{
    state::{RETRO_ROWS, Rect, State, Target},
    weather,
};
use time::OffsetDateTime;

const NAVY: u32 = 0x070d3a;
const PANEL_TOP: u32 = 0x2a3fb8;
const PANEL_BOTTOM: u32 = 0x14227e;
const FRAME: u32 = 0x9fb2ff;
const YELLOW: u32 = 0xf2cf2e;
const WHITE: u32 = 0xf2f2f2;
const CYAN: u32 = 0x7fd8ff;
const ORANGE: u32 = 0xe8892a;
/// Seconds each ticker message stays before the next one cuts in.
const TICKER_SECONDS: i128 = 8;
/// The tracking wobble shown when Home appears.
const GLITCH_SECONDS: f32 = 0.24;
const LIST_X: f32 = 700.;
const LIST_W: f32 = 1068.;
const ROW_Y: f32 = 236.;
const ROW_PITCH: f32 = 74.;

fn rgb(value: u32) -> [f32; 4] {
    c(value, 1.)
}

fn mix(a: u32, b: u32, t: f32) -> [f32; 4] {
    let (a, b) = (rgb(a), rgb(b));
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        1.,
    ]
}

/// A framed info panel: a vertical blue gradient, a light bevel frame and a dark inner line.
fn panel(out: &mut Scene, rect: Rect) {
    const BANDS: usize = 12;
    let band = rect.h / BANDS as f32;
    for index in 0..BANDS {
        out.shape(
            Rect::new(rect.x, rect.y + index as f32 * band, rect.w, band + 0.5),
            0.,
            mix(PANEL_TOP, PANEL_BOTTOM, index as f32 / (BANDS - 1) as f32),
        );
    }
    out.border(rect, 0., 3., c(FRAME, 0.9));
    out.border(rect.inset(3.), 0., 2., c(0x000000, 0.45));
}

/// Broadcast lettering: a heavy black outline and drop shadow under the coloured text.
fn outlined(out: &mut Scene, label: &str, rect: Rect, size: f32, color: u32, center: bool) {
    let style = text(size, 800, 0.);
    let w = (size / 16.).clamp(2., 6.);
    for (dx, dy) in [(w + 2., w + 2.), (-w, -w), (w, -w), (-w, w), (w, w)] {
        out.text(
            label,
            Rect::new(rect.x + dx, rect.y + dy, rect.w, rect.h),
            style,
            c(0x000000, 0.95),
            center,
        );
    }
    out.text(label, rect, style, rgb(color), center);
}

/// Smaller lettering with only the hard drop shadow.
fn shadowed(out: &mut Scene, label: &str, rect: Rect, size: f32, color: [f32; 4], right: bool) {
    let style = if right {
        text(size, 800, 0.).right()
    } else {
        text(size, 800, 0.)
    };
    out.text(
        label,
        Rect::new(rect.x + 3., rect.y + 3., rect.w, rect.h),
        style,
        c(0x000000, 0.8),
        false,
    );
    out.text(label, rect, style, color, false);
}

fn upper(value: &str) -> String {
    value.to_uppercase()
}

/// The label and value of one menu row.
fn row_text(state: &State, target: &Target) -> (String, String) {
    let language = state.preferences.language;
    match target {
        Target::App(id) => (
            state
                .inventory
                .apps
                .iter()
                .find(|app| &app.id == id)
                .map_or_else(String::new, |app| app.title.clone()),
            String::new(),
        ),
        Target::Nav(_) => (
            language.t("retroAllApps").into(),
            state.drawer_apps().len().to_string(),
        ),
        Target::InputsMenu => (
            language.t("inputs").into(),
            state
                .inventory
                .inputs
                .iter()
                .filter(|input| state.preferences.input_visible(input))
                .count()
                .to_string(),
        ),
        Target::LumaSettings => (language.t("launcherSettings").into(), "LUMA".into()),
        Target::TvSettings => (language.t("deviceSettings").into(), "TV".into()),
        _ => (String::new(), String::new()),
    }
}

/// A retro menu page: Home ("Main menu") or All apps, as framed panels with a numbered list.
pub(super) fn page(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let language = state.preferences.language;
    out.shape(Rect::new(0., 0., 1920., 1080.), 0., rgb(NAVY));
    // Faint diagonal broadcast stripes behind the header.
    for index in 0..14 {
        out.shape(
            Rect::new(index as f32 * 150. - 40., 24., 60., 150.),
            0.,
            c(0xffffff, 0.025),
        );
    }

    // Header: only the title; the clock and date sit in the ticker's label block.
    let title_box = Rect::new(96., 36., 1728., 140.);
    panel(out, title_box);
    outlined(
        out,
        &upper(language.t(if state.page == crate::state::Page::Apps {
            "retroAllApps"
        } else {
            "retroMainMenu"
        })),
        Rect::new(132., 66., 1200., 80.),
        64.,
        YELLOW,
        false,
    );
    // Main panel: the selected item on the left, the menu on the right.
    panel(out, Rect::new(96., 200., 1728., 740.));
    preview_box(out, state, assets);
    menu(out, state, assets, now);
    ticker(out, state, assets, now);
    glitch(out, state);
    scanlines(out);
    tv_faults(out, now, state.preferences.reduced_motion);
}

/// The "now selected" box: the focused row's app card or icon, framed like a picture on an
/// old broadcast monitor, with its channel number and name.
fn preview_box(out: &mut Scene, state: &State, assets: &Assets) {
    let frame = Rect::new(150., 252., 470., 620.);
    out.shape(frame, 14., c(0x0b1660, 0.55));
    out.border(frame, 14., 5., rgb(ORANGE));
    let rows = state.retro_rows();
    let Some(index) = rows.iter().position(|row| *row == state.focus) else {
        return;
    };
    let target = &rows[index];
    outlined(
        out,
        &format!("CH {:02}", index % RETRO_ROWS + 1),
        Rect::new(frame.x, frame.y + 28., frame.w, 70.),
        52.,
        YELLOW,
        true,
    );
    let screen = Rect::new(
        frame.x + 35.,
        frame.y + 150.,
        400.,
        400. * FACE_HEIGHT / 270.,
    );
    // Monitor bezel: a heavy black frame with a light bevel, the picture inset behind glass.
    out.shape(screen.inset(-14.), 10., rgb(0x000000));
    out.border(screen.inset(-14.), 10., 3., c(FRAME, 0.9));
    let app = match target {
        Target::App(id) => state.inventory.apps.iter().find(|app| &app.id == id),
        _ => None,
    };
    if let Some(app) = app {
        super::app_face(out, assets, app, screen, state.preferences.language, 6.);
    } else {
        out.shape(screen, 6., mix(PANEL_TOP, PANEL_BOTTOM, 0.5));
        let icon = match target {
            Target::Nav(_) => "grid",
            Target::InputsMenu => "input-hdmi",
            Target::LumaSettings => "luma-mark",
            _ => "tv",
        };
        let size = screen.h * 0.62;
        let rect = Rect::new(
            screen.x + (screen.w - size) / 2.,
            screen.y + (screen.h - size) / 2.,
            size,
            size,
        );
        out.icon(
            assets,
            icon,
            Rect::new(rect.x + 5., rect.y + 5., rect.w, rect.h),
            c(0x000000, 0.9),
        );
        out.icon(assets, icon, rect, rgb(YELLOW));
    }
    // CRT picture: denser scanlines and a soft glare across the top of the screen.
    let mut y = screen.y + 1.;
    while y < screen.y + screen.h {
        out.shape(Rect::new(screen.x, y, screen.w, 1.5), 0., c(0x000000, 0.18));
        y += 3.;
    }
    out.shape(
        Rect::new(screen.x, screen.y, screen.w, screen.h * 0.35),
        6.,
        c(0xffffff, 0.05),
    );
    let (label, _) = row_text(state, target);
    let label = upper(&label);
    let (label, style) = assets.fit_line(&label, text(40., 800, 0.), frame.w - 48., 26.);
    out.text(
        label.clone(),
        Rect::new(frame.x + 3., frame.y + 523., frame.w, 56.),
        style,
        c(0x000000, 0.8),
        true,
    );
    out.text(
        label,
        Rect::new(frame.x, frame.y + 520., frame.w, 56.),
        style,
        rgb(WHITE),
        true,
    );
}

/// Whether the blinking cursor is in its visible half-second.
fn cursor_on(now: OffsetDateTime) -> bool {
    (now.unix_timestamp_nanos() / super::RETRO_BLINK_NANOS) % 2 == 0
}

fn menu(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let rows = state.retro_rows();
    let focused = rows.iter().position(|row| *row == state.focus);
    let page = focused.unwrap_or(0) / RETRO_ROWS;
    let pages = rows.len().div_ceil(RETRO_ROWS);
    let first = page * RETRO_ROWS;
    for (slot, target) in rows.iter().enumerate().skip(first).take(RETRO_ROWS) {
        let y = ROW_Y + (slot - first) as f32 * ROW_PITCH;
        let rect = Rect::new(LIST_X, y, LIST_W, ROW_PITCH - 8.);
        let active = Some(slot) == focused;
        let moving = active && state.moving.is_some();
        if active {
            out.shape(rect, 6., c(YELLOW, if moving { 0.32 } else { 0.16 }));
            out.border(rect, 6., 3., rgb(YELLOW));
            if cursor_on(now) || moving {
                shadowed(
                    out,
                    if moving { "↕" } else { "▶" },
                    Rect::new(rect.x - 52., y + 10., 40., 50.),
                    40.,
                    rgb(YELLOW),
                    false,
                );
            }
        }
        let (label, value) = row_text(state, target);
        shadowed(
            out,
            &format!("{:02}", slot - first + 1),
            Rect::new(rect.x + 22., y + 13., 80., 46.),
            38.,
            rgb(CYAN),
            false,
        );
        let label = upper(&label);
        let value_width = if value.is_empty() {
            0.
        } else {
            assets.width(&value, text(38., 800, 0.)) + 40.
        };
        let (label, style) = assets.fit_line(
            &label,
            text(38., 800, 0.),
            rect.w - 140. - value_width - 24.,
            26.,
        );
        out.text(
            label.clone(),
            Rect::new(rect.x + 123., y + 16., rect.w, 46.),
            style,
            c(0x000000, 0.8),
            false,
        );
        out.text(
            label,
            Rect::new(rect.x + 120., y + 13., rect.w, 46.),
            style,
            rgb(if active { YELLOW } else { WHITE }),
            false,
        );
        if !value.is_empty() {
            shadowed(
                out,
                &upper(&value),
                Rect::new(rect.x, y + 13., rect.w - 24., 46.),
                38.,
                rgb(YELLOW),
                true,
            );
        }
        out.control(target.clone(), rect, false);
    }
    if pages > 1 {
        shadowed(
            out,
            &format!("{} / {}", page + 1, pages),
            Rect::new(
                LIST_X,
                ROW_Y + RETRO_ROWS as f32 * ROW_PITCH + 2.,
                LIST_W - 24.,
                36.,
            ),
            28.,
            c(WHITE, 0.75),
            true,
        );
    }
}

fn ticker(out: &mut Scene, state: &State, assets: &Assets, now: OffsetDateTime) {
    let language = state.preferences.language;
    let bar = Rect::new(0., 964., 1920., 80.);
    out.shape(bar, 0., rgb(0x0a1550));
    out.shape(Rect::new(0., bar.y, 1920., 3.), 0., c(FRAME, 0.9));
    out.shape(
        Rect::new(0., bar.y + bar.h - 3., 1920., 3.),
        0.,
        c(FRAME, 0.9),
    );
    let label = Rect::new(0., bar.y + 3., 460., bar.h - 6.);
    out.shape(label, 0., rgb(ORANGE));
    let clock = upper(&language.clock(
        now,
        state.preferences.hour_24,
        state.preferences.show_seconds,
    ));
    let date = upper(&state.preferences.date_format.format(language, now));
    let clock_style = assets.fit_text(&clock, text(34., 800, 0.), 344.);
    let date_style = assets.fit_text(&date, text(22., 800, 0.), 344.);
    for (value, style, y) in [
        (clock, clock_style, bar.y + 9.),
        (date, date_style, bar.y + 47.),
    ] {
        out.text(
            value.clone(),
            Rect::new(98., y + 2., 344., style.size * 1.3),
            style,
            c(0x000000, 0.8),
            false,
        );
        out.text(
            value,
            Rect::new(96., y, 344., style.size * 1.3),
            style,
            rgb(WHITE),
            false,
        );
    }
    let mut messages = vec![upper(language.t("retroHint"))];
    if let Some(place) = state
        .preferences
        .weather_location
        .as_ref()
        .filter(|_| state.preferences.show_weather)
        && let Some(reading) = state.weather.reading(place)
    {
        messages.push(upper(&format!(
            "{}: {}, {}",
            place.name,
            weather::format_temperature(reading.temperature, state.preferences.temperature_unit),
            language.t(weather::condition(reading).key)
        )));
    }
    let index =
        (now.unix_timestamp_nanos() / (TICKER_SECONDS * 1_000_000_000)) as usize % messages.len();
    shadowed(
        out,
        &messages[index],
        Rect::new(500., bar.y + 18., 1324., 46.),
        36.,
        rgb(WHITE),
        false,
    );
}

/// Bands of tracking noise rolling up once while Home appears; nothing when motion is reduced.
fn glitch(out: &mut Scene, state: &State) {
    let Some(started) = state.page_glide() else {
        return;
    };
    let t = started.elapsed().as_secs_f32() / GLITCH_SECONDS;
    if t >= 1. {
        return;
    }
    let mut seed = 0x9e37_79b9_u32 ^ (t * 1000.) as u32;
    let mut random = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 1000) as f32 / 1000.
    };
    let center = 1180. - t * 1280.;
    for line in 0..18 {
        let y = center - 70. + line as f32 * 8.;
        let mut x = 0.;
        while x < 1920. {
            let w = 30. + random() * 220.;
            out.shape(
                Rect::new(x, y, w, 4. + random() * 4.),
                0.,
                c(0xffffff, (0.15 + random() * 0.45) * (1. - t * 0.6)),
            );
            x += w + random() * 120.;
        }
    }
}

/// CRT scanlines: one faint dark line every four logical pixels, static.
fn scanlines(out: &mut Scene) {
    let mut y = 2.;
    while y < 1080. {
        out.shape(Rect::new(0., y, 1920., 1.5), 0., c(0x000000, 0.12));
        y += 4.;
    }
}

/// Brightness of a fill, to tell focused (light) glass from ordinary panels.
fn luminance(color: [f32; 4]) -> f32 {
    0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]
}

/// Retro TV look for a Liquid Glass page (the launcher Settings): the page keeps its layout,
/// controls and navigation, while its draws are restyled from `from` on. The wallpaper becomes
/// the navy screen, glass becomes framed blue panels, the focused (light) glass turns yellow,
/// soft shadows and glass rims go, corners square up, and lettering turns bold with a hard
/// drop shadow. Scanlines finish the page.
pub(super) fn restyle(out: &mut Scene, assets: &Assets, from: usize) {
    use crate::renderer::Draw;
    let corner = |radius: f32| radius.min(6.);
    let draws = std::mem::take(&mut out.draw);
    let mut restyled = Vec::with_capacity(draws.len() * 2);
    for (index, draw) in draws.into_iter().enumerate() {
        if index < from {
            restyled.push(draw);
            continue;
        }
        match draw {
            Draw::Image(_, rect, ..) | Draw::TintedImage(_, rect, ..)
                if rect.w >= 1920. && rect.h >= 1080. =>
            {
                restyled.push(Draw::Shape(rect, 0., 0., rgb(NAVY)));
            }
            Draw::Glass(rect, radius, tint) => {
                if luminance(tint) > 0.6 {
                    restyled.push(Draw::Shape(rect, corner(radius), 0., rgb(YELLOW)));
                } else {
                    restyled.push(Draw::RoundedGradient(
                        rect,
                        corner(radius),
                        rgb(PANEL_TOP),
                        rgb(PANEL_BOTTOM),
                    ));
                    restyled.push(Draw::Shape(rect, corner(radius), 2., c(FRAME, 0.85)));
                }
            }
            Draw::Rim(rect, radius, ..) => {
                restyled.push(Draw::Shape(rect, corner(radius), 3., rgb(YELLOW)));
            }
            Draw::Shadow(..) => {}
            Draw::Shape(rect, radius, width, color) => {
                // Light hairlines around glass read as the panel frame already.
                if width > 0. && width < 2. && luminance(color) > 0.8 {
                    continue;
                }
                restyled.push(Draw::Shape(rect, corner(radius), width, color));
            }
            Draw::RoundedGradient(rect, radius, top, bottom) => {
                restyled.push(Draw::RoundedGradient(rect, corner(radius), top, bottom));
            }
            Draw::Text(label, rect, style, color, center) => {
                // Bold lettering is wider: shrink it to the box the page laid out, so
                // nothing that fitted before (the clock, values) gets cut.
                let mut bold = style;
                bold.weight = 800;
                // Only single-line boxes: paragraphs wrap instead.
                let single_line = rect.h < 2. * style.size * style.line_height;
                let style =
                    if single_line && rect.w > 0. && assets.width(&label, bold) > rect.w - 1. {
                        let fitted = assets.fit_text(&label, bold, rect.w - 1.);
                        if assets.width(&label, fitted) <= rect.w - 1. {
                            fitted
                        } else {
                            style
                        }
                    } else {
                        bold
                    };
                if luminance(color) > 0.5 {
                    restyled.push(Draw::Text(
                        label.clone(),
                        Rect::new(rect.x + 2., rect.y + 2., rect.w, rect.h),
                        style,
                        [0., 0., 0., 0.8 * color[3]],
                        center,
                    ));
                }
                restyled.push(Draw::Text(label, rect, style, color, center));
            }
            other => restyled.push(other),
        }
    }
    out.draw = restyled;
    scanlines(out);
}

/// Seconds for the hum bar to roll once from the top of the picture to the bottom.
const HUM_SECONDS: f64 = 9.;
/// Glitches come at most once per window, at a random moment inside it, for a short burst.
const GLITCH_WINDOW: f64 = 12.;
const GLITCH_BURST: f64 = 0.3;

/// SplitMix-style hash: the same window always glitches at the same moment and place.
fn hash(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit(value: u64) -> f32 {
    (value % 10_000) as f32 / 10_000.
}

/// Old-TV picture faults over a retro page: a faint hum bar rolling down the picture and, now
/// and then, a short burst of tracking noise with a brightness flicker. Everything follows the
/// clock, so captures are reproducible; nothing is drawn with Reduce motion. Only these draws
/// change between frames, so partial redraw keeps the per-frame work to the bar's strip.
pub(super) fn tv_faults(out: &mut Scene, now: OffsetDateTime, reduced_motion: bool) {
    if reduced_motion {
        return;
    }
    let seconds = now.unix_timestamp_nanos() as f64 / 1e9;
    // Hum bar: a soft band brightening the lines it passes, 240 px tall with a smooth profile.
    let travel = 1080. + 480.;
    let center = ((seconds / HUM_SECONDS).fract() * travel - 240.) as f32;
    const STRIPS: usize = 12;
    for strip in 0..STRIPS {
        let offset = (strip as f32 + 0.5) / STRIPS as f32 * 2. - 1.;
        let alpha = 0.045 * (1. - offset * offset);
        out.shape(
            Rect::new(0., center + offset * 120. - 10., 1920., 20.5),
            0.,
            c(0xffffff, alpha),
        );
    }
    // Occasional tracking burst.
    let window = (seconds / GLITCH_WINDOW).floor();
    let seed = hash(window as u64);
    let start = window * GLITCH_WINDOW + f64::from(unit(seed)) * (GLITCH_WINDOW - 1.);
    let t = seconds - start;
    if !(0. ..GLITCH_BURST).contains(&t) {
        return;
    }
    let mut state = hash(seed ^ (t * 30.) as u64);
    let mut random = move || {
        state = hash(state);
        unit(state)
    };
    // A brief overall flicker, then a band of noisy streaks.
    out.shape(
        Rect::new(0., 0., 1920., 1080.),
        0.,
        c(0xffffff, 0.02 + 0.03 * random()),
    );
    let band = 120. + random() * 760.;
    for line in 0..10 {
        let y = band + line as f32 * 7.;
        let mut x = random() * 200.;
        while x < 1920. {
            let w = 20. + random() * 180.;
            out.shape(
                Rect::new(x, y, w, 2. + random() * 3.),
                0.,
                c(0xffffff, 0.1 + random() * 0.35),
            );
            x += w + random() * 160.;
        }
    }
}
