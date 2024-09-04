use std::sync::{mpsc, Mutex};

use bevy::prelude::*;

/// A channel sender resource, used to send events to Bevy.
#[derive(Clone, Resource, Deref, DerefMut)]
pub struct ChannelSender<T>(mpsc::Sender<T>);

/// A channel receiver resource, used to receive events from Bevy. In most circumstances, you should
/// not need to access this directly, as events are automatically forwarded to the event writer.
#[derive(Resource, Deref, DerefMut)]
struct ChannelReceiver<T>(Mutex<mpsc::Receiver<T>>);

pub trait ChannelAppExtension {
    /// Add a channel to the app, allowing asynchronous tasks to send events to Bevy.
    fn add_channel<T: Event>(&mut self) -> &mut Self;
}

impl ChannelAppExtension for App {
    fn add_channel<T: Event>(&mut self) -> &mut Self {
        assert!(
            !self.world().contains_resource::<ChannelReceiver<T>>(),
            "this event channel is already initialized",
        );
        let (tx, rx) = mpsc::channel::<T>();
        self.insert_resource(ChannelSender(tx))
            .insert_resource(ChannelReceiver(Mutex::new(rx)))
            .add_event::<T>()
            .add_systems(First, process_inbound_channel::<T>)
    }
}

/// Read events from the channel and send them to the event writer.
fn process_inbound_channel<T: Event>(rx: Res<ChannelReceiver<T>>, mut writer: EventWriter<T>) {
    let events = rx.lock().unwrap();
    writer.send_batch(events.try_iter());
}
