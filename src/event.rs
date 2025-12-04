#[derive(Debug, serde::Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub struct Event {
    pub event: EventInner,
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum EventInner {
    TurnOn,
    TurnOff,

    SetBrightness(u8),

    ShowText {
        duration_secs: u32,
        text: String,
        x: u8,
        y: u8,
    },

    ShowPreset {
        name: String,
        duration_s: u64,
        c1: Option<u8>,
        c2: Option<u8>,
        c3: Option<u8>,
        sx: Option<u8>,
        ix: Option<u8>,
    },

    Json {
        value: serde_json::Value,
        sleep_s: u64,
    },
}

#[cfg(test)]
mod tests {
    use crate::event::Event;
    use crate::event::EventInner;

    #[test]
    fn test_turn_on() {
        let e = Event {
            event: EventInner::TurnOn,
        };
        insta::assert_json_snapshot!(e, @r#"
        {
          "event": "TurnOn"
        }
        "#);
    }

    #[test]
    fn test_turn_off() {
        let e = Event {
            event: EventInner::TurnOff,
        };
        insta::assert_json_snapshot!(e, @r#"
        {
          "event": "TurnOff"
        }
        "#);
    }

    #[test]
    fn test_set_brightness() {
        let e = Event {
            event: EventInner::SetBrightness(20),
        };
        insta::assert_json_snapshot!(e, @r#"
        {
          "event": {
            "SetBrightness": 20
          }
        }
        "#);
    }

    #[test]
    fn test_set_show_text() {
        let e = Event {
            event: EventInner::ShowText {
                duration_secs: 10,
                text: String::from("Hello"),
                x: 1,
                y: 1,
            },
        };
        insta::assert_json_snapshot!(e, @r#"
        {
          "event": {
            "ShowText": {
              "duration_secs": 10,
              "text": "Hello",
              "x": 1,
              "y": 1
            }
          }
        }
        "#);
    }

    #[test]
    fn test_deser_testfile_effect() {
        let s = include_str!("../test/effect.json");
        let _: Event = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_deser_testfile_set_brightness_20() {
        let s = include_str!("../test/set_brightness_20.json");
        let _: Event = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_deser_testfile_show_hello() {
        let s = include_str!("../test/show_hello.json");
        let _: Event = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_deser_testfile_turn_off() {
        let s = include_str!("../test/turn_off.json");
        let _: Event = serde_json::from_str(s).unwrap();
    }

    #[test]
    fn test_deser_testfile_turn_on() {
        let s = include_str!("../test/turn_on.json");
        let _: Event = serde_json::from_str(s).unwrap();
    }
}
