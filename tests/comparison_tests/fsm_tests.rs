use rknx::utils::fsm::Fsm;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum State {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Event {
    Connect,
    ConnectSuccess,
    Disconnect,
}

#[test]
fn test_fsm_transitions() {
    let mut transitions = HashMap::new();

    // Disconnected -> Connect -> Connecting
    let mut disc_trans = HashMap::new();
    disc_trans.insert(Event::Connect, State::Connecting);
    transitions.insert(State::Disconnected, disc_trans);

    // Connecting -> ConnectSuccess -> Connected
    let mut conn_trans = HashMap::new();
    conn_trans.insert(Event::ConnectSuccess, State::Connected);
    transitions.insert(State::Connecting, conn_trans);

    // Connected -> Disconnect -> Disconnected
    let mut active_trans = HashMap::new();
    active_trans.insert(Event::Disconnect, State::Disconnected);
    transitions.insert(State::Connected, active_trans);

    // Tracking state transitions with thread-safe list
    let history = Arc::new(Mutex::new(Vec::new()));
    let history_clone = history.clone();

    let mut fsm = Fsm::new(
        State::Disconnected,
        transitions,
        move |new_state, old_state| {
            let mut hist = history_clone.lock().unwrap();
            hist.push((old_state, new_state));
        },
    );

    assert_eq!(fsm.state(), State::Disconnected);

    // Invalid transition: Try to go Connecting -> ConnectSuccess while disconnected
    assert!(!fsm.transition(Event::ConnectSuccess));
    assert_eq!(fsm.state(), State::Disconnected);

    // Valid transition: Disconnected -> Event::Connect -> Connecting
    assert!(fsm.transition(Event::Connect));
    assert_eq!(fsm.state(), State::Connecting);

    // Valid transition: Connecting -> Event::ConnectSuccess -> Connected
    assert!(fsm.transition(Event::ConnectSuccess));
    assert_eq!(fsm.state(), State::Connected);

    // Test forcing state
    fsm.force_state(State::Disconnected);
    assert_eq!(fsm.state(), State::Disconnected);

    // Verify history of transitions
    let hist = history.lock().unwrap();
    assert_eq!(hist.len(), 2);
    assert_eq!(hist[0], (State::Disconnected, State::Connecting));
    assert_eq!(hist[1], (State::Connecting, State::Connected));
}
