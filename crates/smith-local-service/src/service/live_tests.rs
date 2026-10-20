//! The front's activation identity through chat reloads, without IO.
use super::*;

#[test]
fn a_new_chat_with_the_same_number_gets_a_fresh_front_generation() {
    let mut configuration = crate::tests::config();
    configuration.local.live_text = true;
    let mut service = Service::new(configuration, 77).expect("bounded service");
    let state = local::ChatState {
        identity: smith_domain::run::Identity::new([9; 16]),
        activation: 1,
        next_message: 0,
        read: None,
    };
    service.save_state(Time::ZERO, state, false);
    let first = service.remote_activation();
    service.route_local(Time::ZERO, local::Request::Load);
    let mut next_chat = state;
    next_chat.identity = smith_domain::run::Identity::new([10; 16]);
    service.save_state(Time::ZERO, next_chat, true);
    assert_ne!(service.remote_activation(), first, "the front may miss the idle interval between chats");
}
