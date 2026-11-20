use super::*;

#[test]
fn station_accepts_factory_and_unicast_hex() {
    for (input, expected) in [("ff", 255), ("0A", 10), ("F7", 247), ("1", 1)] {
        let station: Station = input.parse().unwrap();
        assert_eq!(station.value(), expected);
        assert_eq!(station.to_string(), format!("{expected:02X}"));
    }
    for input in ["00", "F8", "FE", "255", "+1", ""] {
        assert!(matches!(
            input.parse::<Station>(),
            Err(Error::InvalidStation)
        ));
    }
}

#[test]
fn matches_capture_of_read_request_and_response() {
    assert_eq!(
        frame(Station::FACTORY, 3, 0x00ff, 1),
        [0xff, 0x03, 0x00, 0xff, 0x00, 0x01, 0xa1, 0xe4],
    );
    let mut captured = [0xff, 0x03, 0x02, 0x00, 0xff, 0xd1, 0xd0];
    assert!(validate_crc(&captured).is_ok());
    assert_eq!(station_word(&captured).unwrap(), 0x00ff);
    captured[4] ^= 1;
    assert!(matches!(validate_crc(&captured), Err(Error::Response(_))));
}

#[test]
fn rejects_valid_crc_echo_for_another_value() {
    let request = frame(Station::FACTORY, 6, 0x00c2, 0x61);
    let echoed = frame(Station::FACTORY, 6, 0x00c2, 0x0161);
    assert!(validate_crc(&echoed).is_ok());
    assert!(matches!(
        verify_echo(&request, &echoed),
        Err(Error::Response(_))
    ));
    assert!(verify_echo(&request, &request).is_ok());
}

#[test]
fn scene_parser_and_encoder_agree_with_vendor_presets() {
    for (name, command) in [
        ("red-steady", 0x11),
        ("yellow-slow", 0x22),
        ("green-fast", 0x33),
        ("alarm-steady", 0x14),
        ("alarm-slow", 0x24),
        ("alarm-fast", 0x34),
        ("white-fast", 0x35),
        ("cyan-slow", 0x26),
        ("purple-steady", 0x17),
        ("blue-steady", 0x18),
        ("buzzer-on", 0x40),
        ("buzzer-off", 0x41),
        ("off", 0x61),
    ] {
        assert_eq!(name.parse::<Scene>().unwrap().command(), command);
    }
    assert!("yellow-with-buzzer".parse::<Scene>().is_err());
}

#[test]
fn preview_resets_after_hold_or_uncertain_apply() {
    let scene = Scene::RedWithBuzzer(Pattern::Steady);
    let hold = Duration::from_millis(250);
    for fail_apply in [false, true] {
        let mut writes = Vec::new();
        let mut waited = None;
        let result = preview_with(
            scene,
            hold,
            |action| {
                writes.push(action);
                if action == scene && fail_apply {
                    Err(Error::Timeout)
                } else {
                    Ok(())
                }
            },
            |delay| waited = Some(delay),
        );
        assert_eq!(writes, [scene, Scene::AllOff]);
        assert_eq!(waited, (!fail_apply).then_some(hold));
        assert_eq!(result.is_err(), fail_apply);
    }
}

#[test]
fn preview_reports_when_reset_cannot_be_confirmed() {
    let result = preview_with(
        Scene::Light(Color::Yellow, Pattern::Steady),
        Duration::from_millis(100),
        |scene| {
            if scene == Scene::AllOff {
                Err(Error::Timeout)
            } else {
                Ok(())
            }
        },
        |_| {},
    );
    assert!(matches!(
        result,
        Err(Error::ResetUnconfirmed { apply: None, .. })
    ));
}
