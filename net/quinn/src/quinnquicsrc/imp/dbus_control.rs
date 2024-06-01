// https://github.com/diwic/dbus-rs/blob/master/dbus-tokio/examples/tokio_server_cr.rs

use dbus::channel::MatchingReceiver;
use dbus::message::MatchRule;
use dbus_crossroads::Crossroads;
use dbus_tokio::connection;
use futures::future;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::sleep;

use crate::quinnquicsrc::imp::Settings;
use crate::utils::make_socket_addr;

// This is our "State" object that we are going to store inside the crossroads instance.
struct State {
    called_count: u32,
    settings: Arc<Mutex<Settings>>,
}

pub async fn init(settings: Arc<Mutex<Settings>>) {
    let _ = run(settings).await;
}

pub async fn run(settings: Arc<Mutex<Settings>>) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the D-Bus session bus (this is blocking, unfortunately).
    let (resource, c) = connection::new_session_sync()?;

    // The resource is a task that should be spawned onto a tokio compatible
    // reactor ASAP. If the resource ever finishes, you lost connection to D-Bus.
    //
    // To shut down the connection, both call _handle.abort() and drop the connection.
    let _handle = tokio::spawn(async {
        let err = resource.await;
        panic!("Lost connection to D-Bus: {}", err);
    });

    // Let's request a name on the bus, so that clients can find us.
    c.request_name("org.freedesktop.gst", false, true, false)
        .await?;

    // Create a new crossroads instance.
    // The instance is configured so that introspection and properties interfaces
    // are added by default on object path additions.
    let mut cr = Crossroads::new();

    // Enable async support for the crossroads instance.
    cr.set_async_support(Some((
        c.clone(),
        Box::new(|x| {
            tokio::spawn(x);
        }),
    )));

    // Let's build a new interface, which can be used for "State" objects.
    let iface_token = cr.register("org.freedesktop.gst", |b| {
        // This row is just for introspection: It advertises that we
        // can send a RebindHappened signal.  We use the single-tuple
        // to say that we have one single argument, named "addr" of
        // type "String".
        b.signal::<(String,), _>("RebindHappened", ("addr",));
        // Let's add a method to the interface. We have the method
        // name, followed by names of input and output arguments (used
        // for introspection). The closure then controls the types of
        // these arguments. The last argument to the closure is a
        // tuple of the input arguments.
        b.method_with_cr_async(
            "Rebind",
            ("addr",),
            ("reply",),
            |mut ctx, cr, (addr,): (String,)| {
                let state: &mut State = cr.data_mut(ctx.path()).unwrap(); // ok_or_else(|| MethodErr::no_path(ctx.path()))?;
                                                                          // And here's what happens when the method is called.
                println!("Incoming Rebind call with addr {}!", addr);
                state.called_count += 1;
                let s = format!(
                    "Hello! This API has been used {} times.",
                    state.called_count
                );
                {
                    let mut settings = state.settings.lock().unwrap();
                    settings.migration_trigger_size = 1;
                    settings.next_addr.clone_from(&addr);
                }
                async move {
                    if let Err(error) = make_socket_addr(addr.as_str()) {
                        return ctx.reply(Ok((format!("{error}"),)));
                    }
                    // Let's wait half a second just to show off how async we are.
                    sleep(Duration::from_millis(500)).await;
                    // The ctx parameter can be used to conveniently send extra messages.
                    let signal_msg = ctx.make_signal("RebindHappened", (addr,));
                    ctx.push_msg(signal_msg);
                    // And the return value is a tuple of the output arguments.
                    ctx.reply(Ok((s,)))
                    // The reply is sent when ctx is dropped / goes out of scope.
                }
            },
        );
    });

    // Let's add the "/quic" path, which implements the
    // org.freedesktop.gst interface, to the crossroads instance.
    cr.insert(
        "/quic",
        &[iface_token],
        State {
            called_count: 0,
            settings,
        },
    );

    // We add the Crossroads instance to the connection so that
    // incoming method calls will be handled.
    c.start_receive(
        MatchRule::new_method_call(),
        Box::new(move |msg, conn| {
            cr.handle_message(msg, conn).unwrap();
            true
        }),
    );

    // Run forever.
    future::pending::<()>().await;
    unreachable!()
}
