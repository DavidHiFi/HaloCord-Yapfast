//! HaloCord's flat microphone ([`crate::halocord::set_flat`]), as TestCord's
//! stereo loader sends it: in stereo, past echo cancellation, the rumble
//! filter, noise suppression, gain control and leveling. Only its volume
//! applies, which never pushes a peak into clipping, and the gate or push to
//! talk. The gate opens on the level alone, since nothing here tells speech
//! from noise.
//!
//! Yapfast's own capture and meter stop when flat sound comes on, and these
//! stop when it goes off, so a call or the microphone test switches within a
//! block and opens the microphone again in stereo or mono.

use super::*;

/// Whether the microphone goes flat.
pub(super) fn on() -> bool {
    crate::halocord::flat()
}

/// A block in interleaved stereo, as flat sound reads and sends it.
const STEREO: usize = BLOCK * CHANNELS as usize;

/// How long it waits before opening a microphone again that went away
/// mid-call.
const RETRY: Duration = Duration::from_secs(1);

/// Runs a call's capture until stopped: flat while flat sound is on, and
/// Yapfast's own otherwise. Returns the denoiser Yapfast's used last, or
/// `denoiser` if it never ran.
pub(super) fn run(request: CaptureRequest, mut denoiser: Option<Denoiser>) -> Option<Denoiser> {
    let mut first = true;
    while !request.stop.load(Ordering::Relaxed) {
        let flat = on();
        let opened = if flat {
            send(&request)
        } else {
            denoiser = request.run(denoiser);
            // Yapfast's ends early only when the microphone didn't open.
            request.stop.load(Ordering::Relaxed) || on()
        };
        if !opened {
            // The call already heard why.
            if first {
                return denoiser;
            }
            tracing::warn!(
                flat,
                "the Discord microphone didn't open again; trying again"
            );
            wait(&request.stop, RETRY);
        }
        first = false;
    }
    denoiser
}

/// Measures the microphone for [`GateMeter`] until it stops: flat while flat
/// sound is on, and through Yapfast's own measure otherwise.
pub(super) fn measure(
    microphone: Option<&str>,
    shared: &MeterShared,
    playback: Option<SyncSender<[i16; BLOCK]>>,
) -> Result<()> {
    while !shared.stop.load(Ordering::Relaxed) {
        if on() {
            measure_flat(microphone, shared, playback.as_ref())?;
        } else {
            super::measure(microphone, shared, playback.clone())?;
        }
    }
    Ok(())
}

/// Sleeps for `time`, or until `stop`.
fn wait(stop: &AtomicBool, time: Duration) {
    let step = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    while waited < time && !stop.load(Ordering::Relaxed) {
        thread::sleep(step);
        waited += step;
    }
}

/// Opens the microphone with both its channels, in 10 ms blocks, to open
/// again if it goes away and comes back.
fn open(microphone: Option<&str>) -> Result<LastingInput<i16>> {
    LastingInput::open(
        microphone,
        "Discord microphone",
        SAMPLE_RATE,
        CHANNELS,
        BLOCK_TIME,
    )
}

/// Sends the microphone flat until the call stops or flat sound goes off,
/// and says whether it opened.
fn send(request: &CaptureRequest) -> bool {
    let mut stream = match open(request.microphone.as_deref()) {
        Ok(stream) => {
            let _ = request.opened.send(Ok(()));
            stream
        }
        Err(error) => {
            let _ = request.opened.send(Err(error));
            return false;
        }
    };
    tracing::info!("the Discord microphone opens flat, in stereo");
    let reverse = request.reverse.lock().unwrap();
    let mut processing = Flat::default();
    let mut block = [0i16; STEREO];
    while !request.stop.load(Ordering::Relaxed) && on() {
        stream.read(&mut block);
        // Flat sound cancels no echo, so what played only goes back.
        while let Ok(played) = reverse.try_recv() {
            recycle(played);
        }
        let was_open = processing.gate.is_open();
        let sent = processing.block(&mut block, crate::volume::microphone());
        // The gate sends while it's open.
        let open = !sent.is_empty();
        if open != was_open {
            request.talking.store(open, Ordering::Relaxed);
            let _ = if open {
                request.out.clear();
                request.track.play()
            } else {
                request.track.pause()
            };
        }
        for samples in sent {
            request.out.push_stereo(samples);
        }
    }
    request.talking.store(false, Ordering::Relaxed);
    let _ = request.track.pause();
    true
}

/// Measures the flat microphone until the meter stops or flat sound goes
/// off. The level falls 90 dB a second, as Yapfast's does.
fn measure_flat(
    microphone: Option<&str>,
    shared: &MeterShared,
    playback: Option<&SyncSender<[i16; BLOCK]>>,
) -> Result<()> {
    if playback.is_some() {
        crate::audio_io::raise_capture_priority();
    }
    let mut stream = open(microphone)?;
    let mut processing = Flat::default();
    let mut block = [0i16; STEREO];
    let mut shown = f32::NEG_INFINITY;
    let mut silent = 0u32;
    while !shared.stop.load(Ordering::Relaxed) && on() {
        stream.read(&mut block);
        // Nothing at all from the device, as a muted or wrong one gives.
        silent = if block.iter().all(|sample| *sample == 0) {
            silent.saturating_add(1)
        } else {
            0
        };
        processing.block(&mut block, crate::volume::microphone());
        // Heard even in push to talk, or with the gate shut.
        if let Some(playback) = playback
            && !shared.stop.load(Ordering::Relaxed)
        {
            let _ = playback.try_send(downmix(&block));
        }
        shared.silent_blocks.store(silent, Ordering::Relaxed);
        // One block's fall.
        shown = processing.level.max(shown - 0.9);
        shared.level.store(shown.to_bits(), Ordering::Relaxed);
        shared
            .open
            .store(processing.gate.is_open(), Ordering::Relaxed);
    }
    Ok(())
}

/// The flat microphone's state across blocks.
struct Flat {
    gate: Gate<STEREO>,
    talk: crate::push_to_talk::Talk,
    /// Sets the volume only; it levels nothing.
    leveler: crate::leveling::Leveler,
    /// The last block's level as the gate heard it, in RMS dBFS.
    level: f32,
    stereo: [f32; STEREO],
}

impl Default for Flat {
    fn default() -> Self {
        Flat {
            gate: Gate::default(),
            talk: Default::default(),
            leveler: Default::default(),
            level: f32::NEG_INFINITY,
            stereo: [0.0; STEREO],
        }
    }
}

impl Flat {
    /// Takes 10 ms of interleaved stereo microphone, sets it to `volume` in
    /// place, and returns the blocks to send, as f32 in full scale.
    fn block(&mut self, block: &mut [i16; STEREO], volume: f32) -> &[[f32; STEREO]] {
        let talking = self.talking();
        self.level = decibels(block) + 20.0 * volume.log10();
        for (value, sample) in self.stereo.iter_mut().zip(block.iter()) {
            *value = f32::from(*sample);
        }
        // Both channels' samples count toward the rate, so the volume's ramp
        // spans the block.
        self.leveler.process(
            &mut self.stereo,
            SAMPLE_RATE * u32::from(CHANNELS),
            false,
            volume,
        );
        for (sample, value) in block.iter_mut().zip(self.stereo.iter_mut()) {
            *value = value.clamp(-32768.0, 32767.0);
            *sample = *value as i16;
            *value /= 32768.0;
        }
        match talking {
            Some(sending) => self.gate.pass(&self.stereo, sending),
            None => {
                self.gate.sensitivity = crate::noise::sensitivity();
                self.gate.block(&self.stereo, self.level, true)
            }
        }
    }

    /// Push to talk's say for this block, as [`Processing`] works it out for
    /// a pair: `Some` sends or holds back whatever the gate hears, and `None`
    /// leaves it to the gate. A push-to-mute key held, or a fake deafen,
    /// sends nothing in either mode.
    fn talking(&mut self) -> Option<bool> {
        let muting = crate::push_to_talk::muting() || crate::halocord::fake_deafened();
        if crate::push_to_talk::on() {
            let held = self.talk.block(
                crate::push_to_talk::held(),
                crate::push_to_talk::release_delay_ms(),
            );
            Some(held && !muting)
        } else {
            muting.then_some(false)
        }
    }
}

/// Both channels of a stereo block in one, for the microphone test.
fn downmix(block: &[i16; STEREO]) -> [i16; BLOCK] {
    let mut mono = [0i16; BLOCK];
    for (sample, pair) in mono.iter_mut().zip(block.chunks_exact(2)) {
        *sample = ((i32::from(pair[0]) + i32::from(pair[1])) / 2) as i16;
    }
    mono
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wave() -> [i16; STEREO] {
        let mut block = [0i16; STEREO];
        for (index, pair) in block.chunks_exact_mut(2).enumerate() {
            let wave = ((index as f32 * 0.05).sin() * 8000.0) as i16;
            pair[0] = wave;
            pair[1] = -wave / 2;
        }
        block
    }

    #[test]
    fn flat_sound_keeps_both_channels_as_they_came() {
        let mut flat = Flat::default();
        let came = wave();
        let mut block = came;
        flat.block(&mut block, 1.0);
        assert_eq!(block, came, "at full volume nothing changes");
        assert!(
            flat.level > -20.0,
            "the level is the block's: {}",
            flat.level
        );
    }

    #[test]
    fn the_volume_lowers_both_channels_and_the_level() {
        let mut flat = Flat::default();
        let came = wave();
        let mut block = came;
        for _ in 0..10 {
            block = came;
            flat.block(&mut block, 0.5);
        }
        let quieter = |now: i16, was: i16| now.unsigned_abs() <= was.unsigned_abs();
        assert!(
            block
                .iter()
                .zip(&came)
                .all(|(now, was)| quieter(*now, *was))
        );
        assert!(block.iter().zip(&came).any(|(now, was)| now != was));
        let full = decibels(&came);
        assert!(
            (flat.level - (full - 6.02)).abs() < 0.1,
            "{} {full}",
            flat.level
        );
    }

    #[test]
    fn the_test_hears_both_channels_in_one() {
        let came = wave();
        let mono = downmix(&came);
        assert_eq!(
            mono[1],
            ((i32::from(came[2]) + i32::from(came[3])) / 2) as i16
        );
    }

    #[test]
    fn a_stereo_gate_sends_whole_stereo_blocks() {
        let mut gate = Gate::<STEREO>::default();
        let mut block = [0.0; STEREO];
        block[1] = 0.5;
        for _ in 0..WARM_UP {
            assert!(gate.block(&block, -20.0, true).is_empty());
        }
        for _ in 1..OPEN_AFTER {
            assert!(gate.block(&block, -20.0, true).is_empty());
        }
        let sent = gate.block(&block, -20.0, true);
        assert!(!sent.is_empty());
        assert!(
            sent.iter()
                .all(|samples| samples[0] == 0.0 && samples[1] == 0.5)
        );
    }

    /// A fake deafen sends nothing, as a deafened Discord client sends
    /// nothing; a fake mute leaves the microphone as it is.
    #[test]
    fn a_fake_deafen_sends_nothing() {
        let _alone = crate::halocord::FAKE_VOICE_TESTS
            .lock()
            .unwrap_or_else(|held| held.into_inner());
        let mut flat = Flat::default();
        crate::halocord::set_fake_voice(false, true);
        let deafened = flat.talking();
        crate::halocord::set_fake_voice(true, false);
        let muted = flat.talking();
        crate::halocord::set_fake_voice(false, false);
        let neither = flat.talking();
        // Push to talk's own switches are global too; with them at rest the
        // gate decides (None).
        let at_rest = !crate::push_to_talk::on() && !crate::push_to_talk::muting();
        assert_eq!(deafened, Some(false), "nothing is sent");
        if at_rest {
            assert_eq!(muted, None, "a fake mute leaves it to the gate");
            assert_eq!(neither, None, "and so does neither");
        }
    }
}
