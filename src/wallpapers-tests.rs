use super::*;
use crate::companion::TOKEN_ALPHABET;
use std::{
    net::{Ipv4Addr, TcpListener},
    sync::atomic::AtomicUsize,
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "luma-wallpapers-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn wallpapers(&self) -> PathBuf {
        self.0.join("wallpapers")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn image(width: u32, height: u32) -> Vec<u8> {
    jpeg(
        &image::RgbImage::from_pixel(width, height, image::Rgb([150, 200, 230])),
        85,
    )
    .unwrap()
}

/// The HTTP tests use only a loopback mock, never TV or weather services.
fn http(path: &Path, request: impl FnOnce(&str) -> Vec<u8>) -> (String, bool) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let address = listener.local_addr().unwrap();
    let path = path.to_owned();
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        serve(&mut socket, &address.to_string(), "secret", &path, || true).unwrap_or(false)
    });
    let mut socket = TcpStream::connect(address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    socket.write_all(&request(&address.to_string())).unwrap();
    let mut response = String::new();
    let _ = socket.read_to_string(&mut response);
    (response, worker.join().unwrap())
}

#[test]
fn generated_ids_are_safe_and_selection_survives_preferences_reload() {
    let id = format!("custom-{}", random_id().unwrap());
    assert!(valid_id(&id));
    assert_ne!(id, format!("custom-{}", random_id().unwrap()));
    let mut preferences = crate::model::Preferences {
        background: id.clone(),
        ..Default::default()
    };
    preferences.sanitize();
    assert_eq!(preferences.background, id);
    for invalid in [
        "custom-../etc",
        "custom-",
        "custom-123",
        "custom-FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        "/etc/passwd",
    ] {
        assert!(!valid_id(invalid));
        preferences.background = invalid.into();
        preferences.sanitize();
        assert_eq!(preferences.background, "dunes");
    }
}

#[test]
fn upload_tokens_are_eight_unbiased_alphanumeric_characters() {
    let tokens: Vec<_> = (0..256).map(|_| random_token().unwrap()).collect();
    for token in &tokens {
        assert_eq!(token.len(), 8);
        assert!(token.bytes().all(|byte| TOKEN_ALPHABET.contains(&byte)));
    }
    let distinct: std::collections::HashSet<_> = tokens.iter().collect();
    assert_eq!(distinct.len(), tokens.len());
}

#[test]
fn uploads_retain_full_resolution_unshaded_images_and_discover_legacy_fallbacks() {
    let fixture = Fixture::new();
    let item = store(&fixture.wallpapers(), &image(1920, 1080), || true).unwrap();
    assert!(item.has_original);
    let original = image::open(item.unshaded_image()).unwrap().to_rgb8();
    let shaded = image::open(item.image()).unwrap().to_rgb8();
    assert_eq!(original.dimensions(), (1920, 1080));
    assert_ne!(original.get_pixel(960, 1000), shaded.get_pixel(960, 1000));
    assert!(catalog(&fixture.wallpapers())[0].has_original);
    fs::remove_file(item.directory.join("original.jpg")).unwrap();
    let legacy = catalog(&fixture.wallpapers());
    assert_eq!(legacy.len(), 1);
    assert!(!legacy[0].has_original);
    assert_eq!(legacy[0].unshaded_image(), legacy[0].image());
}

#[test]
fn deletion_protects_selected_and_bundled_backgrounds_and_only_removes_the_requested_upload() {
    let fixture = Fixture::new();
    let bytes = image(1920, 1080);
    let first = store(&fixture.wallpapers(), &bytes, || true).unwrap();
    let second = store(&fixture.wallpapers(), &bytes, || true).unwrap();
    assert!(delete_wallpaper(&fixture.wallpapers(), &first.id, &first.id).is_err());
    for id in ["dunes", "../preferences.json", "custom-../../other"] {
        assert!(delete_wallpaper(&fixture.wallpapers(), id, "dunes").is_err());
    }
    assert_eq!(catalog(&fixture.wallpapers()).len(), 2);
    delete_wallpaper(&fixture.wallpapers(), &first.id, &second.id).unwrap();
    assert!(!first.directory.exists());
    assert!(second.directory.join("wallpaper.jpg").exists());
    assert_eq!(catalog(&fixture.wallpapers()).len(), 1);
    assert!(delete_wallpaper(&fixture.wallpapers(), &first.id, &second.id).is_err());
}

#[cfg(unix)]
#[test]
fn deletion_rejects_symlink_roots_and_entries_and_never_follows_internal_symlinks() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let root = fixture.wallpapers();
    fs::create_dir(&root).unwrap();
    let outside = fixture.0.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), b"keep").unwrap();
    let id = format!("custom-{}", "1".repeat(32));
    symlink(&outside, root.join(&id)).unwrap();
    assert!(delete_wallpaper(&root, &id, "dunes").is_err());
    fs::remove_file(root.join(&id)).unwrap();
    fs::create_dir(root.join(&id)).unwrap();
    symlink(&outside, root.join(&id).join("linked")).unwrap();
    let linked_root = fixture.0.join("linked-root");
    symlink(&root, &linked_root).unwrap();
    assert!(delete_wallpaper(&linked_root, &id, "dunes").is_err());
    delete_wallpaper(&root, &id, "dunes").unwrap();
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
}

#[test]
fn worker_reports_deletion_before_the_updated_catalog() {
    let fixture = Fixture::new();
    let item = store(&fixture.wallpapers(), &image(1920, 1080), || true).unwrap();
    let service = Service::new(&fixture.0.join("preferences.json"), true, false).unwrap();
    assert!(
        matches!(service.receiver.recv_timeout(Duration::from_secs(5)).unwrap(), Event::Catalog(items) if items.len() == 1)
    );
    service.delete(item.id.clone(), "dunes".into());
    assert!(
        matches!(service.receiver.recv_timeout(Duration::from_secs(5)).unwrap(), Event::Deleted(id, Ok(())) if id == item.id)
    );
    assert!(
        matches!(service.receiver.recv_timeout(Duration::from_secs(5)).unwrap(), Event::Catalog(items) if items.is_empty())
    );
    assert!(!item.directory.exists());
}

#[test]
fn uploads_are_unnamed_normalized_immutable_and_discoverable_after_restart() {
    let fixture = Fixture::new();
    let bytes = image(1920, 1080);
    let first = store(&fixture.wallpapers(), &bytes, || true).unwrap();
    let second = store(&fixture.wallpapers(), &bytes, || true).unwrap();
    assert_ne!(first.id, second.id);
    let found = catalog(&fixture.wallpapers());
    assert_eq!(found.len(), 2);
    assert!(found.iter().any(|item| item.id == first.id));
    let background = image::open(first.image()).unwrap();
    let thumbnail = image::open(first.thumbnail()).unwrap();
    assert_eq!((background.width(), background.height()), (1920, 1080));
    assert_eq!((thumbnail.width(), thumbnail.height()), (480, 270));
    assert!(
        background.to_rgb8().get_pixel(960, 1000)[0] < thumbnail.to_rgb8().get_pixel(240, 250)[0]
    );
    assert_eq!(
        fs::read(first.directory.join("metadata.json"))
            .unwrap()
            .len(),
        serde_json::to_vec(&first).unwrap().len()
    );
}

#[test]
fn validation_and_cancellation_leave_no_partial_gallery_entries() {
    let fixture = Fixture::new();
    let root = fixture.wallpapers();
    assert!(store(&root, b"<svg></svg>", || true).is_err());
    assert!(store(&root, &image(320, 240), || true).is_err());
    assert!(store(&root, &image(1920, 1080), || false).is_err());
    assert!(catalog(&root).is_empty());
    assert!(!root.exists());
    let mut reader = Cursor::new(b"unfinished");
    assert!(
        read_exact(
            &mut reader,
            &mut [0; 20],
            &|| false,
            Instant::now() + REQUEST_TIME
        )
        .is_err()
    );
    assert!(read_exact(&mut reader, &mut [0; 20], &|| true, Instant::now()).is_err());
}

#[test]
fn cancellation_before_commit_cleans_only_the_new_staging_files() {
    let fixture = Fixture::new();
    let checks = AtomicUsize::new(0);
    let result = store(&fixture.wallpapers(), &image(1920, 1080), || {
        checks.fetch_add(1, Ordering::Relaxed) == 0
    });
    assert!(result.is_err());
    assert_eq!(checks.load(Ordering::Relaxed), 2);
    assert!(fs::read_dir(fixture.wallpapers()).unwrap().next().is_none());
}

#[test]
fn gallery_capacity_is_checked_before_decoding_or_creating_another_upload() {
    let fixture = Fixture::new();
    create_directory(&fixture.wallpapers()).unwrap();
    for index in 0..MAX_WALLPAPERS {
        let id = format!("custom-{index:032x}");
        let directory = fixture.wallpapers().join(&id);
        create_directory(&directory).unwrap();
        write_new(
            &directory.join("metadata.json"),
            &serde_json::to_vec(&Wallpaper {
                id,
                version: None,
                directory: directory.clone(),
                has_original: false,
            })
            .unwrap(),
        )
        .unwrap();
        write_new(&directory.join("wallpaper.jpg"), b"fixture").unwrap();
        write_new(&directory.join("thumbnail.jpg"), b"fixture").unwrap();
    }
    assert_eq!(catalog(&fixture.wallpapers()).len(), MAX_WALLPAPERS);
    assert!(
        store(&fixture.wallpapers(), b"not decoded", || true)
            .unwrap_err()
            .contains("full")
    );
    assert_eq!(
        fs::read_dir(fixture.wallpapers()).unwrap().count(),
        MAX_WALLPAPERS
    );
}

#[test]
fn decoder_enforces_transport_dimensions_and_composites_transparent_pngs() {
    assert!(decode(&vec![0; MAX_BODY + 1]).is_err());
    let rgba = image::RgbaImage::from_pixel(1920, 1080, image::Rgba([255, 0, 0, 0]));
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    assert_eq!(
        decode(png.get_ref()).unwrap().get_pixel(0, 0).0,
        [23, 25, 28]
    );
    let too_tall = image::RgbaImage::new(1, 12000);
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(too_tall)
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    assert!(decode(png.get_ref()).is_err());
}

#[test]
fn protocol_rejects_oversized_ambiguous_and_chunked_requests() {
    for request_text in [
        format!(
            "POST / HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY + 1
        ),
        "POST / HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n".into(),
        "POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n".into(),
        "POST / HTTP/1.1\r\nContent-Length: -1\r\n\r\n".into(),
        "POST / HTTP/1.1\r\nContent-Length: +1\r\n\r\n".into(),
        "POST / HTTP/1.1\r\nContent-Length: 9999999999999999999999999\r\n\r\n".into(),
        format!("GET / HTTP/1.1\r\nX-Header: {}", "a".repeat(8192)),
    ] {
        assert!(
            request(
                &mut Cursor::new(request_text),
                &|| true,
                Instant::now() + REQUEST_TIME
            )
            .is_err()
        );
    }
}

#[test]
fn only_the_session_origin_host_token_and_upload_header_allow_writes() {
    let fixture = Fixture::new();
    for (route, headers, expected) in [
        ("/wrong/", "", "404 Not Found"),
        ("/secret/../../etc/passwd", "", "404 Not Found"),
        (
            "/secret/",
            "Origin: https://evil.example\r\n",
            "403 Forbidden",
        ),
        (
            "/secret/",
            "Sec-Fetch-Site: cross-site\r\n",
            "403 Forbidden",
        ),
    ] {
        let (response, changed) = http(&fixture.wallpapers(), |host| {
            format!("GET {route} HTTP/1.1\r\nHost: {host}\r\n{headers}\r\n").into_bytes()
        });
        assert!(
            response.starts_with(&format!("HTTP/1.1 {expected}")),
            "{response}"
        );
        assert!(!changed);
    }
    let (response, changed) = http(&fixture.wallpapers(), |_| {
        b"GET /secret/ HTTP/1.1\r\nHost: attacker.example\r\n\r\n".to_vec()
    });
    assert!(response.starts_with("HTTP/1.1 403"));
    assert!(!changed);
    let (response, changed) = http(&fixture.wallpapers(), |host| {
        format!("POST /secret/upload HTTP/1.1\r\nHost: {host}\r\nContent-Length: 0\r\nContent-Type: image/jpeg\r\n\r\n").into_bytes()
    });
    assert!(response.starts_with("HTTP/1.1 403"));
    assert!(!changed);
    assert!(!fixture.wallpapers().exists());
}

#[test]
fn slashless_session_urls_redirect_to_the_local_asset_base() {
    let fixture = Fixture::new();
    let (response, changed) = http(&fixture.wallpapers(), |host| {
        format!("GET /secret HTTP/1.1\r\nHost: {host}\r\n\r\n").into_bytes()
    });
    assert!(response.starts_with("HTTP/1.1 303 See Other\r\n"));
    assert!(response.contains("\r\nLocation: /secret/\r\n"));
    assert!(response.contains("Cache-Control: no-store"));
    assert!(!changed);
    for (target, headers, expected) in [
        ("/wrong", "", "404 Not Found"),
        (
            "/secret",
            "Origin: https://evil.example\r\n",
            "403 Forbidden",
        ),
    ] {
        let (response, changed) = http(&fixture.wallpapers(), |host| {
            format!("GET {target} HTTP/1.1\r\nHost: {host}\r\n{headers}\r\n").into_bytes()
        });
        assert!(response.starts_with(&format!("HTTP/1.1 {expected}")));
        assert!(!response.contains("\r\nLocation:"));
        assert!(!changed);
    }
}

#[test]
fn browser_assets_are_local_and_a_real_http_upload_needs_no_name() {
    let fixture = Fixture::new();
    for route in [
        "",
        "app.js",
        "crop.js",
        "storage.js",
        "app.css",
        "luma-mark.svg",
        "info.json",
    ] {
        let (response, changed) = http(&fixture.wallpapers(), |host| {
            format!("GET /secret/{route} HTTP/1.1\r\nHost: {host}\r\n\r\n").into_bytes()
        });
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("Cache-Control: no-store"));
        assert!(response.contains("frame-ancestors 'none'"));
        assert!(!changed);
        if route == "luma-mark.svg" {
            assert!(response.starts_with("HTTP/1.1 200"));
            assert!(response.contains("image/svg+xml"));
            assert!(response.contains("<svg"));
        }
        if route == "info.json" {
            assert!(response.starts_with("HTTP/1.1 200"));
            assert!(response.contains(&fixture.wallpapers().to_string_lossy().to_string()));
        }
    }
    let bytes = image(1920, 1080);
    let (response, changed) = http(&fixture.wallpapers(), |host| {
        let mut wire = format!("POST /secret/upload HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nX-Luma-Upload: secret\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", bytes.len()).into_bytes();
        wire.extend_from_slice(&bytes);
        wire
    });
    assert!(response.starts_with("HTTP/1.1 201"), "{response}");
    assert!(changed);
    let items = catalog(&fixture.wallpapers());
    assert_eq!(items.len(), 1);
    assert!(response.contains(&items[0].id));
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(items[0].directory.join("metadata.json")).unwrap())
            .unwrap();
    assert_eq!(metadata, serde_json::json!({"id": items[0].id}));
}

#[test]
fn legacy_named_uploads_keep_their_ids_and_load_without_names() {
    let fixture = Fixture::new();
    let item = store(&fixture.wallpapers(), &image(1920, 1080), || true).unwrap();
    assert!(item.directory.starts_with(fixture.wallpapers()));
    assert!(valid_id(&item.id));
    fs::write(
        item.directory.join("metadata.json"),
        serde_json::to_vec(&serde_json::json!({"id": item.id, "name": "旅行 & sea"})).unwrap(),
    )
    .unwrap();
    let items = catalog(&fixture.wallpapers());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, item.id);
    assert_eq!(items[0].image(), item.image());
}

#[cfg(unix)]
#[test]
fn symlink_storage_and_gallery_entries_are_never_followed() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let elsewhere = fixture.0.join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    symlink(&elsewhere, fixture.wallpapers()).unwrap();
    assert!(store(&fixture.wallpapers(), &image(1920, 1080), || true).is_err());
    assert!(fs::read_dir(&elsewhere).unwrap().next().is_none());
    assert!(catalog(&fixture.wallpapers()).is_empty());
}

#[test]
fn qr_generation_is_deterministic_and_has_a_reasonable_size() {
    let address = "192.168.1.2:12345".parse().unwrap();
    let token = random_token().unwrap();
    let info = session(address, &token).unwrap();
    assert_eq!(info.url, format!("http://192.168.1.2:12345/{token}/"));
    assert_eq!(info.qr.len(), (info.qr_size * info.qr_size) as usize);
    assert!(info.qr_size <= 65);
    assert_eq!(info.qr, session(address, &token).unwrap().qr);
}

#[test]
fn session_closure_revokes_the_socket_and_reopening_rotates_the_capability() {
    let fixture = Fixture::new();
    let mut service = Service::new(&fixture.0.join("preferences.json"), true, false).unwrap();
    assert!(matches!(
        service
            .receiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap(),
        Event::Catalog(_)
    ));
    service.set_active(true);
    let first = match service
        .receiver
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
    {
        Event::Session(generation, Ok(session)) => {
            assert_eq!(generation, service.generation());
            session
        }
        _ => panic!("Expected a local upload session"),
    };
    let address = first
        .url
        .strip_prefix("http://")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    assert!(address.starts_with("127.0.0.1:"));
    service.set_active(false);
    assert!(matches!(
        service
            .receiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap(),
        Event::Ended(_)
    ));
    assert!(TcpStream::connect(address).is_err());
    service.set_active(true);
    let second = match service
        .receiver
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
    {
        Event::Session(_, Ok(session)) => session,
        _ => panic!("Expected a new upload session"),
    };
    assert_ne!(first.url, second.url);
    assert_ne!(first.url.split('/').nth(3), second.url.split('/').nth(3));
    service.set_active(false);
    assert!(matches!(
        service
            .receiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap(),
        Event::Ended(_)
    ));
}

#[test]
fn cancelling_a_slow_request_releases_the_worker_without_waiting_for_the_body() {
    let fixture = Fixture::new();
    let mut service = Service::new(&fixture.0.join("preferences.json"), true, false).unwrap();
    service
        .receiver
        .recv_timeout(Duration::from_secs(3))
        .unwrap();
    service.set_active(true);
    let info = match service
        .receiver
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
    {
        Event::Session(_, Ok(session)) => session,
        _ => panic!("Expected a session"),
    };
    let parts: Vec<_> = info.url.split('/').collect();
    let mut socket = TcpStream::connect(parts[2]).unwrap();
    write!(socket, "POST /{}/upload HTTP/1.1\r\nHost: {}\r\nX-Luma-Upload: {}\r\nContent-Type: image/jpeg\r\nContent-Length: 100000\r\n\r\npartial", parts[3], parts[2], parts[3]).unwrap();
    service.set_active(false);
    assert!(matches!(
        service
            .receiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap(),
        Event::Ended(_)
    ));
    assert!(catalog(&fixture.wallpapers()).is_empty());
}

const BING_REPLY: &str = r#"{"images":[{"startdate":"20261113","fullstartdate":"202611132300",
"url":"/th?id=OHR.BlueHK_DE-DE1662761141_1920x1080.jpg&rf=LaDigue_1920x1080.jpg&pid=hp",
"urlbase":"/th?id=OHR.BlueHK_DE-DE1662761141","copyright":"Victoria Harbour"}],
"tooltips":{"loading":"Wird geladen..."}}"#;

#[test]
fn bing_archive_replies_yield_a_fixed_origin_full_hd_url() {
    assert_eq!(
        parse_bing(BING_REPLY.as_bytes()).unwrap(),
        (
            "20261113".into(),
            "https://www.bing.com/th?id=OHR.BlueHK_DE-DE1662761141_1920x1080.jpg".into()
        )
    );
    for reply in [
        "not json",
        r#"{"images":[]}"#,
        r#"{"images":[{"startdate":"2026111","urlbase":"/th?id=OHR.A"}]}"#,
        r#"{"images":[{"startdate":"../../x1","urlbase":"/th?id=OHR.A"}]}"#,
        r#"{"images":[{"startdate":"20261113","urlbase":"@evil.example/th?id=A"}]}"#,
        r#"{"images":[{"startdate":"20261113","urlbase":"//evil.example/a"}]}"#,
        r#"{"images":[{"startdate":"20261113","urlbase":"/th?id=A B"}]}"#,
    ] {
        assert!(parse_bing(reply.as_bytes()).is_err(), "{reply}");
    }
}

#[test]
fn bing_ids_and_versioned_paths_stay_separate_from_uploads() {
    assert!(valid_id(BING_ID));
    assert!(!valid_custom_id(BING_ID));
    assert!(!valid_id("bing-daily-2"));
    let item = Wallpaper {
        id: BING_ID.into(),
        version: Some("20261113".into()),
        directory: PathBuf::from("/w/bing-daily"),
        has_original: true,
    };
    assert_eq!(item.image(), "/w/bing-daily/wallpaper-20261113.jpg");
    assert_eq!(item.unshaded_image(), "/w/bing-daily/original-20261113.jpg");
    assert_eq!(item.thumbnail(), "/w/bing-daily/thumbnail-20261113.jpg");
    assert!(valid_version("20261113"));
    assert!(!valid_version("2026111a") && !valid_version("../x") && !valid_version(""));
}

#[test]
fn daily_images_replace_the_previous_day_without_taking_a_gallery_slot() {
    let fixture = Fixture::new();
    let directory = fixture.wallpapers();
    install_bing(&directory, "20261112", &image(1920, 1080)).unwrap();
    install_bing(&directory, "20261113", &image(1920, 1080)).unwrap();
    let items = catalog(&directory);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, BING_ID);
    assert_eq!(items[0].version.as_deref(), Some("20261113"));
    assert!(items[0].has_original);
    assert_eq!(image::open(items[0].image()).unwrap().width(), 1920);
    assert_eq!(bing_version(&directory).as_deref(), Some("20261113"));
    // Two days are kept, so a texture still drawn from yesterday's path survives the switch.
    install_bing(&directory, "20261114", &image(1920, 1080)).unwrap();
    let mut files: Vec<_> = fs::read_dir(directory.join(BING_ID))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    assert_eq!(
        files,
        [
            "metadata.json",
            "original-20261113.jpg",
            "original-20261114.jpg",
            "thumbnail-20261113.jpg",
            "thumbnail-20261114.jpg",
            "wallpaper-20261113.jpg",
            "wallpaper-20261114.jpg",
        ]
    );
    // No staging directories remain.
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    // Wrong dimensions or invalid versions never replace the current image.
    assert!(install_bing(&directory, "20261115", &image(1280, 720)).is_err());
    assert!(install_bing(&directory, "../../x", &image(1920, 1080)).is_err());
    assert_eq!(bing_version(&directory).as_deref(), Some("20261114"));
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    // It can never be deleted, even when not selected.
    assert!(delete_wallpaper(&directory, BING_ID, "dunes").is_err());
    assert!(directory.join(BING_ID).is_dir());
    assert_eq!(custom_count(&directory), 0);
}

#[test]
fn the_bing_tile_does_not_count_toward_the_upload_limit() {
    let fixture = Fixture::new();
    let directory = fixture.wallpapers();
    install_bing(&directory, "20261113", &image(1920, 1080)).unwrap();
    for index in 0..MAX_WALLPAPERS - 1 {
        let id = format!("custom-{index:032x}");
        let path = directory.join(&id);
        create_directory(&path).unwrap();
        let item = Wallpaper {
            id,
            version: None,
            directory: path.clone(),
            has_original: false,
        };
        write_new(
            &path.join("metadata.json"),
            &serde_json::to_vec(&item).unwrap(),
        )
        .unwrap();
        write_new(&path.join("wallpaper.jpg"), b"fixture").unwrap();
        write_new(&path.join("thumbnail.jpg"), b"fixture").unwrap();
    }
    assert_eq!(custom_count(&directory), MAX_WALLPAPERS - 1);
    // The 20th upload still fits next to the Bing tile.
    store(&directory, &image(1920, 1080), || true).unwrap();
    let items = catalog(&directory);
    assert_eq!(items.len(), MAX_WALLPAPERS + 1);
    assert_eq!(items[0].id, BING_ID);
    // A full gallery is rejected before decoding, so placeholder bytes suffice.
    assert!(
        store(&directory, b"fixture", || true)
            .unwrap_err()
            .contains("full")
    );
}
