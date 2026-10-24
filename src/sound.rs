//! NoteFetch's four sounds. PLAN.md "A sound palette".
//!
//! Short sine notes rendered into a WAV in memory and handed to Windows' own
//! sound player, the way TweakSound's `tone.rs` does it, so there is no new
//! library. What TweakSound's rounds settled with Laurent, and this keeps: a
//! lead-in of a twelfth of a second, so a Bluetooth link waking up swallows
//! silence and not the note; a quiet volume of 0.15 ("Yes, right volume");
//! and a short fade at each end of every note, so nothing clicks.
//!
//! What is NoteFetch's own: the pitches. StableSound plays E and B (660 and
//! 990 Hz) and A (880 Hz); TweakSound plays B and E (493.88 and 659.25 Hz).
//! Every note here is a C sharp, D, F sharp or G, each at least a whole tone
//! from all of those, so no NoteFetch sound can be heard as half of another
//! app's. A test holds that.
//!
//! Sounds are only for what the user did (CLAUDE.md), with one exception the
//! plan allows: something going wrong is heard even when NoteFetch was doing
//! it by itself. The interface decides which to play; this module only plays.

use std::sync::OnceLock;

use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT, SND_SYNC};
use windows::core::PCWSTR;

use crate::config::Volume;

const SAMPLE_RATE: u32 = 44_100;

/// TweakSound round 3's lead-in, chosen by Laurent: "Maybe worth trying a
/// 12th of a second?"
const LEAD_IN_MS: u32 = 83;

// The notes. Sharps are written out, since `#` cannot go in a name.
const C_SHARP_5: f32 = 554.37;
const D_5: f32 = 587.33;
const D_6: f32 = 1174.66;
const F_SHARP_5: f32 = 739.99;
const F_SHARP_6: f32 = 1479.98;
const G_5: f32 = 783.99;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    /// A change you made worked: a rename, move, delete, skip or folder.
    Done,
    /// Something you started has finished: a sync, a download, a sign-in or
    /// a move to a new output folder.
    Finished,
    /// Something could not be done, or went wrong.
    Refused,
    /// NoteFetch has a question for you.
    Attention,
}

pub const ALL: [Sound; 4] = [Sound::Done, Sound::Finished, Sound::Refused, Sound::Attention];

struct Note {
    /// Hertz. Zero is silence: the sine of zero is zero for every frame.
    freq: f32,
    ms: u32,
}

const fn note(freq: f32, ms: u32) -> Note {
    Note { freq, ms }
}

const fn silence(ms: u32) -> Note {
    Note { freq: 0.0, ms }
}

/// Up a major third, quickly: small and pleased.
const DONE: &[Note] = &[silence(LEAD_IN_MS), note(D_5, 50), note(F_SHARP_5, 80)];

/// Done's two notes and then the octave, so it sounds like Done grown up:
/// the same family, but plainly "that whole job is over".
const FINISHED: &[Note] = &[silence(LEAD_IN_MS), note(D_5, 50), note(F_SHARP_5, 50), note(D_6, 120)];

/// Down a tritone, the interval that sounds most unresolved. Falling, where
/// Done and Finished rise.
const REFUSED: &[Note] = &[silence(LEAD_IN_MS), note(G_5, 70), note(C_SHARP_5, 110)];

/// Up an octave with a gap, like a call: "hey, over here". Neither rising
/// smoothly like Done nor falling like Refused.
const ATTENTION: &[Note] = &[silence(LEAD_IN_MS), note(F_SHARP_5, 70), silence(40), note(F_SHARP_6, 90)];

fn notes(sound: Sound) -> &'static [Note] {
    match sound {
        Sound::Done => DONE,
        Sound::Finished => FINISHED,
        Sound::Refused => REFUSED,
        Sound::Attention => ATTENTION,
    }
}

/// Peak amplitude for each volume. Normal is TweakSound's proven 0.15.
fn amplitude(volume: Volume) -> f32 {
    match volume {
        Volume::Quiet => 0.07,
        Volume::Normal => 0.15,
        Volume::Loud => 0.3,
    }
}

/// Play a sound and return at once.
pub fn play(sound: Sound, volume: Volume) {
    send(wave(sound, volume), SND_ASYNC);
}

/// Play a sound and return when it has finished: for the developer flag,
/// which would otherwise exit before the sound is heard.
pub fn play_and_wait(sound: Sound, volume: Volume) {
    send(wave(sound, volume), SND_SYNC);
}

/// The rendered sound, made the first time it is wanted and kept: with
/// `SND_ASYNC` Windows reads the bytes after `PlaySoundW` returns, so they
/// must live as long as the program, which a static does.
fn wave(sound: Sound, volume: Volume) -> &'static [u8] {
    static WAVES: [[OnceLock<Vec<u8>>; 3]; 4] = [const { [const { OnceLock::new() }; 3] }; 4];
    let s = ALL.iter().position(|a| *a == sound).unwrap_or(0);
    let v = match volume {
        Volume::Quiet => 0,
        Volume::Normal => 1,
        Volume::Loud => 2,
    };
    WAVES[s][v].get_or_init(|| render(notes(sound), amplitude(volume)))
}

fn send(wave: &'static [u8], how: windows::Win32::Media::Audio::SND_FLAGS) {
    // SAFETY: with SND_MEMORY the pointer is the address of a whole WAV file
    // in memory, never read as text, and the bytes are static. SND_NODEFAULT
    // keeps the Windows ding from playing instead if anything is refused:
    // everything a sound marks is also in the Log, so silence is the right
    // failure.
    unsafe {
        let _ = PlaySoundW(PCWSTR(wave.as_ptr() as *const u16), None, SND_MEMORY | SND_NODEFAULT | how);
    }
}

fn frames(note: &Note) -> u64 {
    u64::from(note.ms) * u64::from(SAMPLE_RATE) / 1000
}

/// A complete mono 16-bit WAV file, header and all.
fn render(notes: &[Note], amplitude: f32) -> Vec<u8> {
    let total: u64 = notes.iter().map(frames).sum();
    let data_bytes = (total * 2) as u32;
    let mut wave = Vec::with_capacity(44 + data_bytes as usize);
    wave.extend_from_slice(b"RIFF");
    wave.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wave.extend_from_slice(b"WAVE");
    wave.extend_from_slice(b"fmt ");
    wave.extend_from_slice(&16u32.to_le_bytes()); // length of this chunk
    wave.extend_from_slice(&1u16.to_le_bytes()); // uncompressed PCM
    wave.extend_from_slice(&1u16.to_le_bytes()); // one channel
    wave.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wave.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    wave.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    wave.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wave.extend_from_slice(b"data");
    wave.extend_from_slice(&data_bytes.to_le_bytes());
    for note in notes {
        let length = frames(note);
        for frame in 0..length {
            let phase = frame as f64 * f64::from(note.freq) * std::f64::consts::TAU / f64::from(SAMPLE_RATE);
            let value = phase.sin() as f32 * amplitude * envelope(frame, length);
            wave.extend_from_slice(&((value * f32::from(i16::MAX)) as i16).to_le_bytes());
        }
    }
    wave
}

/// A short fade at each end of a note, since a sine that starts or stops at
/// full strength clicks, and in earphones the click is louder than the note.
fn envelope(frame: u64, length: u64) -> f32 {
    let fade = (length / 8).clamp(1, 240);
    let rising = (frame + 1).min(fade) as f32 / fade as f32;
    let falling = (length - frame).min(fade) as f32 / fade as f32;
    rising.min(falling)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn audible(sound: Sound) -> Vec<f32> {
        notes(sound).iter().filter(|n| n.freq > 0.0).map(|n| n.freq).collect()
    }

    fn samples(wave: &[u8]) -> Vec<i16> {
        wave[44..].as_chunks::<2>().0.iter().map(|p| i16::from_le_bytes(*p)).collect()
    }

    #[test]
    fn the_header_agrees_with_the_samples() {
        for sound in ALL {
            let wave = render(notes(sound), 0.15);
            assert_eq!(&wave[0..4], b"RIFF");
            assert_eq!(&wave[36..40], b"data");
            let declared = u32::from_le_bytes(wave[40..44].try_into().unwrap()) as usize;
            assert_eq!(declared, wave.len() - 44);
            let riff = u32::from_le_bytes(wave[4..8].try_into().unwrap()) as usize;
            assert_eq!(riff, wave.len() - 8);
        }
    }

    #[test]
    fn every_sound_starts_with_the_lead_in_then_sounds() {
        let lead_in = (u64::from(LEAD_IN_MS) * u64::from(SAMPLE_RATE) / 1000) as usize;
        for sound in ALL {
            let s = samples(&render(notes(sound), 0.15));
            assert!(s[..lead_in].iter().all(|x| *x == 0), "{sound:?}");
            assert!(s[lead_in..].iter().any(|x| x.abs() > 1000), "{sound:?}");
        }
    }

    #[test]
    fn louder_is_louder_and_none_clips() {
        let peak = |v| samples(&render(DONE, amplitude(v))).iter().map(|x| x.unsigned_abs()).max().unwrap();
        assert!(peak(Volume::Quiet) < peak(Volume::Normal));
        assert!(peak(Volume::Normal) < peak(Volume::Loud));
        assert!(amplitude(Volume::Loud) <= 0.5);
    }

    #[test]
    fn done_and_finished_rise_and_refused_falls() {
        for sound in [Sound::Done, Sound::Finished, Sound::Attention] {
            let a = audible(sound);
            assert!(a.windows(2).all(|w| w[1] > w[0]), "{sound:?} should rise");
        }
        let r = audible(Sound::Refused);
        assert!(r[1] < r[0]);
        assert_eq!(audible(Sound::Finished)[..2], audible(Sound::Done)[..], "Finished begins as Done");
    }

    /// Semitones between two pitches, ignoring the octave.
    fn apart(a: f32, b: f32) -> f32 {
        let s = (12.0 * (a / b).log2()).rem_euclid(12.0);
        s.min(12.0 - s)
    }

    #[test]
    fn no_note_is_one_another_app_uses() {
        // StableSound's E, B and A, and TweakSound's B and E. Copied, since
        // they are separate programs: if either changes its notes, change these.
        let others = [660.0, 990.0, 880.0, 493.88, 659.25];
        for sound in ALL {
            for freq in audible(sound) {
                for other in others {
                    assert!(apart(freq, other) > 1.5, "{freq} Hz in {sound:?} is close to {other} Hz");
                }
            }
        }
    }

    #[test]
    fn a_note_begins_and_ends_near_silence() {
        let length = 4410;
        assert!(envelope(0, length) < 0.01);
        assert!(envelope(length - 1, length) < 0.01);
        assert!(envelope(length / 2, length) > 0.99);
    }
}
