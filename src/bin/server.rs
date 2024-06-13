use bevy::tasks::TaskPool;
use bevy::{prelude::*, tasks::TaskPoolBuilder};
use bevy_eventwork::{ConnectionId, EventworkRuntime, Network, NetworkData, NetworkEvent};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use bevy_eventwork_mod_websockets::{NetworkSettings, WebSocketProvider};
use websocket_bevy::shared;
use bevy::prelude::*;



//plugins 4 state and event system
use bevy_states::states::StatePlugin;
use bevy_states::states::MachineState;
use bevy_states::events::{send_simple_event, EventTypes, SimpleEvent, SimpleEventPlugin};

#[derive(Resource, Default)]
struct PreviousState(Option<MachineState>);
pub fn main() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::log::LogPlugin::default(), bevy::input::InputPlugin::default()

));
    // Before we can register the potential message types, we
    // need to add the plugin
    app.add_plugins(bevy_eventwork::EventworkPlugin::<
        WebSocketProvider,
        bevy::tasks::TaskPool,
    >::default());

    // Make sure you insert the EventworkRuntime resource with your chosen Runtime
    app.insert_resource(EventworkRuntime(
        TaskPoolBuilder::new().num_threads(2).build(),
    ));

    // A good way to ensure that you are not forgetting to register
    // any messages is to register them where they are defined!
    shared::server_register_network_messages(&mut app);

    app.add_systems(Startup, setup_networking);
    app.add_systems(Update, (handle_connection_events, handle_messages));

    // We have to insert the WS [`NetworkSettings`] with our chosen settings.
    app.insert_resource(NetworkSettings::default());

    //these are for the state and event systems
    app.add_plugins(StatePlugin);
    app.add_plugins(SimpleEventPlugin);


    //state change detection system
    app.insert_resource(PreviousState::default());
    app.add_systems(Update, check_state_change);


    app.run();
}




fn check_state_change(
    current_state: Res<State<MachineState>>,
    mut previous_state: ResMut<PreviousState>,
    net: Res<Network<WebSocketProvider>>,

) {
    let current = current_state.get();
    if previous_state.0.as_ref() != Some(current) {
        println!("state update system reporting: State now is : {:?}", current);
        let message = format!("State has changed to {:?}", current_state);

        net.broadcast(shared::NewChatMessage {
            name: String::from("SERVER"),
            
            message: String::from(message),
        });
        previous_state.0 = Some(current.clone());
    }
}






// On the server side, you need to setup networking. You do not need to do so at startup, and can start listening
// at any time.
fn setup_networking(
    mut net: ResMut<Network<WebSocketProvider>>,
    settings: Res<NetworkSettings>,
    task_pool: Res<EventworkRuntime<TaskPool>>,
) {
    let ip_address = "127.0.0.1".parse().expect("Could not parse ip address");

    info!("Address of the server: {}", ip_address);

    let _socket_address = SocketAddr::new(ip_address, 8080);

    match net.listen(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        &task_pool.0,
        &settings,
    ) {
        Ok(_) => (),
        Err(err) => {
            error!("Could not start listening: {}", err);
            panic!();
        }
    }

    info!("Started listening for new connections!");
}


#[derive(Component)]
struct Player(ConnectionId);

fn handle_connection_events(
    mut commands: Commands,
    net: Res<Network<WebSocketProvider>>,
    mut network_events: EventReader<NetworkEvent>,
) {
    for event in network_events.read() {
        if let NetworkEvent::Connected(conn_id) = event {
            commands.spawn((Player(*conn_id),));

            // Broadcasting sends the message to all connected players! (Including the just connected one in this case)
            net.broadcast(shared::NewChatMessage {
                name: String::from("SERVER"),
                message: format!("New user connected; {}", conn_id),
            });
            info!("New player connected: {}", conn_id);
        }
    }
}

// Receiving a new message is as simple as listening for events of `NetworkData<T>`
fn handle_messages(
    mut state_messages: EventReader<NetworkData<shared::StateChangeMessage>>,
    mut new_messages: EventReader<NetworkData<shared::UserChatMessage>>,
    net: Res<Network<WebSocketProvider>>,
    state: Res<State<MachineState>>,
    mut event_writer: EventWriter<SimpleEvent>,



) {
    for message in new_messages.read() {
        let user = message.source();

        info!("Received message from user: {}", message.message);

        net.broadcast(shared::NewChatMessage {
            name: format!("{}", user),
            message: message.message.clone(),
        });        
    }
    //this is the handler for state messages recieved it takes the request and message and sends a state event with the same request and message
    for message in state_messages.read() {
        let user = message.source();

        info!("Received message from user: {:?}", message.event_type);
        let event_type = convert_event_type(message.event_type.clone());

        net.broadcast(shared::StateChangeMessage {
            event_type: message.event_type.clone(),
        });
        
        send_simple_event(&mut event_writer, event_type);

        //println!("state: {:?}", state);

    }


}

//this feels unneccesary but will state for now
//this maps the button hit from statemessage to the state that needs to be changed
fn convert_event_type(event: shared::EventThatHappened) -> EventTypes {
    match event {
        shared::EventThatHappened::Start => EventTypes::Start,
        shared::EventThatHappened::Stop => EventTypes::Stop,
        shared::EventThatHappened::Emergency => EventTypes::Emergency,
        shared::EventThatHappened::PauseButtonHit => EventTypes::PauseButtonHit,
        shared::EventThatHappened::Power => EventTypes::Power,
    }
}





