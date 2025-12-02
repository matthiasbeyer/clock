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
}
