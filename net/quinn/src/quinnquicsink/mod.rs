// Copyright (C) 2024, Asymptotic Inc.
//      Author: Sanchayan Maity <sanchayan@asymptotic.io>
//G
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

/**
 * element-quinnquicsink:
 * @short-description: Send data over the network via QUIC
 *
 * ## Example sender pipeline
 * ```bash
 * gst-launch-1.0 -v -e audiotestsrc num-buffers=512 ! \
 * audio/x-raw,format=S16LE,rate=48000,channels=2,layout=interleaved ! opusenc ! \
 * quinnquicsink server-name="quic.net" bind-address="127.0.0.1" bind-port=6001 \
 * address="127.0.0.1" port=6000 certificate-file="certificates/fullchain.pem" \
 * private-key-file="certificates/privkey.pem"
 * ```
 */
use gst::glib;
use gst::prelude::*;

use tracing_subscriber::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct QuinnQuicSink(ObjectSubclass<imp::QuinnQuicSink>) @extends gst_base::BaseSink, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    tracing_log::LogTracer::init().expect("Failed to set logger");
    let env_filter = tracing_subscriber::EnvFilter::try_from_env("QUINNQUICSINK_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_ids(true)
        .with_target(true)
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::NEW
                | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
        );
    let subscriber = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(fmt_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    gst::Element::register(
        Some(plugin),
        "quinnquicsink",
        gst::Rank::MARGINAL,
        QuinnQuicSink::static_type(),
    )
}
