use std::collections::HashMap;
use std::hash::Hash;

pub struct Fsm<State, Event>
where
    State: Eq + Hash + Clone + Send + Sync + 'static,
    Event: Eq + Hash + Clone + Send + Sync + 'static,
{
    current_state: State,
    transitions: HashMap<State, HashMap<Event, State>>,
    on_state_change: Box<dyn Fn(State, State) + Send + Sync + 'static>,
}

impl<State, Event> Fsm<State, Event>
where
    State: Eq + Hash + Clone + Send + Sync + 'static,
    Event: Eq + Hash + Clone + Send + Sync + 'static,
{
    pub fn new<F>(
        initial_state: State,
        transitions: HashMap<State, HashMap<Event, State>>,
        on_state_change: F,
    ) -> Self
    where
        F: Fn(State, State) + Send + Sync + 'static,
    {
        Self {
            current_state: initial_state,
            transitions,
            on_state_change: Box::new(on_state_change),
        }
    }

    pub fn state(&self) -> State {
        self.current_state.clone()
    }

    pub fn transition(&mut self, event: Event) -> bool {
        if let Some(next_state) = self.transitions
            .get(&self.current_state)
            .and_then(|t| t.get(&event))
        {
            let old_state = self.current_state.clone();
            let next_state = next_state.clone();
            self.current_state = next_state.clone();

            // Run callback
            (self.on_state_change)(next_state, old_state);
            true
        } else {
            false
        }
    }

    pub fn force_state(&mut self, state: State) {
        self.current_state = state;
    }
}
