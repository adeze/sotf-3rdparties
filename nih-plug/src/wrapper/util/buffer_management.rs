//! Helpers for safely constructing [`Buffer`]s from a plugin host's audio buffers.

use std::num::NonZeroU32;
use std::ptr::NonNull;

use crate::prelude::{AudioIOLayout, Buffer};

/// Return the number of host buses declared for one side of an audio layout.
pub(crate) fn audio_layout_bus_count(layout: AudioIOLayout, is_input: bool) -> usize {
    let has_main_bus = if is_input {
        layout.main_input_channels.is_some()
    } else {
        layout.main_output_channels.is_some()
    };
    let auxiliary_buses = if is_input {
        layout.aux_input_ports.len()
    } else {
        layout.aux_output_ports.len()
    };

    usize::from(has_main_bus) + auxiliary_buses
}

/// Return a declared bus width by host-visible index, with the main bus first when present.
pub(crate) fn audio_layout_bus_channels(
    layout: AudioIOLayout,
    is_input: bool,
    bus_index: usize,
) -> Option<usize> {
    let main_channels = if is_input {
        layout.main_input_channels
    } else {
        layout.main_output_channels
    };
    let auxiliary_ports = if is_input {
        layout.aux_input_ports
    } else {
        layout.aux_output_ports
    };

    if let Some(channels) = main_channels {
        if bus_index == 0 {
            return Some(channels.get() as usize);
        }

        auxiliary_ports
            .get(bus_index - 1)
            .map(|channels| channels.get() as usize)
    } else {
        auxiliary_ports
            .get(bus_index)
            .map(|channels| channels.get() as usize)
    }
}

/// Buffers created using [`create_buffers`]. At some point the main `Plugin::process()` should
/// probably also take an argument like this instead of main+aux buffers if we also want to provide
/// access to overflowing input channels for e.g. stereo to mono plugins.
pub struct Buffers<'a, 'buffer: 'a> {
    pub main_buffer: &'a mut Buffer<'buffer>,

    // We can't use `AuxiliaryBuffers` here directly because we need different lifetimes for `'a`
    // and `'buffer` while `AuxiliaryBuffers` uses the same lifetime for both.
    pub aux_inputs: &'a mut [Buffer<'buffer>],
    pub aux_outputs: &'a mut [Buffer<'buffer>],
}

/// A helper for safely creating and initializing [`Buffer`]s based on the host's input and output
/// buffers.
pub struct BufferManager {
    // These are the storage backing the fields in `BufferSource`. The wrapper needs to set these
    // values to match the channel pointers provided by the host. If audio buffers are not provided
    // for a bus, then they should be set to `None`. This helper will then copy data to the buffers
    // or fill them with zeroes if there is no data, while also accounting for in-place main IO
    // buffers.
    main_input_channel_pointers: Option<ChannelPointers>,
    main_output_channel_pointers: Option<ChannelPointers>,
    aux_input_channel_pointers: Vec<Option<ChannelPointers>>,
    aux_output_channel_pointers: Vec<Option<ChannelPointers>>,

    /// Number of channels declared by the selected primary input layout. This is kept separately
    /// from the callback's pointers so plugins can distinguish a complete input from a host that
    /// supplied only a prefix (or no primary input at all).
    main_input_channels: usize,

    /// The backing buffers that will be filled during `create_buffers`. This `'static` lifetime
    /// will be shortened when returning a reference to these buffers in `create_buffers` to match
    /// the function's lifetime.
    main_buffer: Buffer<'static>,
    /// Prepared storage for main input channels that have no corresponding main output channel.
    main_input_storage: Vec<Vec<f32>>,

    aux_input_buffers: Vec<Buffer<'static>>,
    /// Stores the data to back `aux_input_buffers`. We need to copy the host's auxiliary input
    /// buffers to our own first because the `Buffer` API is designed around mutable buffers, and
    /// the host may reuse its input buffers between plugins.
    aux_input_storage: Vec<Vec<Vec<f32>>>,

    aux_output_buffers: Vec<Buffer<'static>>,
}

// SAFETY: The raw pointers in the `ChannelPointers` fields/vectors are only used as scratch storage
//         inside of the `create_buffers()` function.
unsafe impl Send for BufferManager {}
unsafe impl Sync for BufferManager {}

/// Host data that the plugin's [`Buffer`]s should be created from. Leave these fields as `None`
/// values
#[derive(Debug)]
pub struct BufferSource<'a> {
    pub main_input_channel_pointers: &'a mut Option<ChannelPointers>,
    pub main_output_channel_pointers: &'a mut Option<ChannelPointers>,
    pub aux_input_channel_pointers: &'a mut [Option<ChannelPointers>],
    pub aux_output_channel_pointers: &'a mut [Option<ChannelPointers>],
}

/// Pointers to raw multichannel audio data for this port.
#[derive(Debug, Clone, Copy)]
pub struct ChannelPointers {
    /// A raw pointer to an array of f32 arrays, containing one array for each channel. `ptrs` must
    /// contain (at least) `num_channel` `*const f32`s, and each of those inner arrays must contain
    /// (at least) `num_samples` `f32` values.
    pub ptrs: NonNull<*mut f32>,
    /// The number of audio channels used for this port.
    pub num_channels: usize,
}

impl BufferManager {
    /// Initialize managed buffers for a specific audio IO layout. The actual buffers can be set up
    /// using channel pointer data using [`create_buffers()`][Self::create_buffers()].
    pub fn for_audio_io_layout(max_buffer_size: usize, audio_io_layout: AudioIOLayout) -> Self {
        let main_input_channels = audio_io_layout
            .main_input_channels
            .map(NonZeroU32::get)
            .unwrap_or(0) as usize;
        let main_output_channels = audio_io_layout
            .main_output_channels
            .map(NonZeroU32::get)
            .unwrap_or(0) as usize;
        let main_buffer_channels = main_input_channels.max(main_output_channels);

        // The buffers are preallocated so that `create_buffers()` can be called without having to
        // allocate. Trailing main input channels are backed by prepared scratch storage when the
        // declared input width exceeds the output width.
        let mut main_buffer = Buffer::default();
        unsafe {
            main_buffer.set_slices(0, |output_slices| {
                output_slices.resize_with(main_buffer_channels, || &mut []);
            })
        };
        let main_input_storage = (0..main_input_channels.saturating_sub(main_output_channels))
            .map(|_| vec![0.0; max_buffer_size])
            .collect();

        let mut aux_input_buffers = Vec::with_capacity(audio_io_layout.aux_input_ports.len());
        let mut aux_input_storage = Vec::with_capacity(audio_io_layout.aux_input_ports.len());
        for num_channels in audio_io_layout.aux_input_ports {
            let mut buffer = Buffer::default();
            unsafe {
                buffer.set_slices(0, |slices| {
                    slices.resize_with(num_channels.get() as usize, || &mut []);
                })
            };

            aux_input_buffers.push(buffer);
            aux_input_storage.push(vec![
                vec![0.0; max_buffer_size];
                num_channels.get() as usize
            ]);
        }

        let mut aux_output_buffers = Vec::with_capacity(audio_io_layout.aux_output_ports.len());
        for num_channels in audio_io_layout.aux_output_ports {
            let mut buffer = Buffer::default();
            unsafe {
                buffer.set_slices(0, |slices| {
                    slices.resize_with(num_channels.get() as usize, || &mut []);
                })
            };

            aux_output_buffers.push(buffer);
        }

        Self {
            main_input_channel_pointers: None,
            main_output_channel_pointers: None,
            aux_input_channel_pointers: vec![None; audio_io_layout.aux_input_ports.len()],
            aux_output_channel_pointers: vec![None; audio_io_layout.aux_output_ports.len()],

            main_input_channels,

            main_buffer,
            main_input_storage,

            aux_input_buffers,
            aux_input_storage,

            aux_output_buffers,
        }
    }

    /// Initialize the buffers using the host provided buffer pointers and return a reference to the
    /// created buffers that can be passed to `Plugin::process()`. This accounts for in-place main
    /// IO, missing channel pointers, null pointers, and mismatching channel counts. All
    /// uninitialized buffer data (aux outputs, and main output channels with no matching input
    /// channel) are filled with zeroes.
    ///
    /// `sample_offset` and `num_samples` can be used to slice a set of host channel pointers for
    /// sample accurate automation. If any of the outputs are missing because the host hasn't
    /// provided enough channels or outputs, then they will be replaced by empty slices.
    ///
    /// # Panics
    ///
    /// May panic if one of the inner channel pointers is a null pointer.
    ///
    /// # Safety
    ///
    /// Any provided `ChannelPointers` must point to memory regions that remain valid to read from
    /// or write to for the lifetime of the returned [`Buffers`]. Input channel regions must be
    /// mutually disjoint, output channel regions must be mutually disjoint, and any input/output
    /// alias must be an exact same-channel alias. Cross-channel or partial overlaps are not
    /// supported.
    pub unsafe fn create_buffers<'a, 'buffer: 'a>(
        &'a mut self,
        sample_offset: usize,
        num_samples: usize,
        set_buffer_sources: impl FnOnce(&mut BufferSource),
    ) -> Buffers<'a, 'buffer> {
        // Make sure the caller can't forget to unset previously set values
        self.main_input_channel_pointers = None;
        self.main_output_channel_pointers = None;
        self.aux_input_channel_pointers.fill(None);
        self.aux_output_channel_pointers.fill(None);
        set_buffer_sources(&mut BufferSource {
            main_input_channel_pointers: &mut self.main_input_channel_pointers,
            main_output_channel_pointers: &mut self.main_output_channel_pointers,
            aux_input_channel_pointers: &mut self.aux_input_channel_pointers,
            aux_output_channel_pointers: &mut self.aux_output_channel_pointers,
        });

        let main_output_width = self
            .main_buffer
            .channels()
            .saturating_sub(self.main_input_storage.len());
        let main_input_storage = &mut self.main_input_storage;

        // Main output channels point directly to host output pointers. A wider main input gets a
        // scratch-backed suffix so all declared input channels remain available to the in-place
        // plugin API without exposing those extra channels as host outputs.
        self.main_buffer.set_slices(num_samples, |main_slices| {
            match self.main_output_channel_pointers {
                Some(output_channel_pointers) => {
                    nih_debug_assert_eq!(
                        main_output_width,
                        output_channel_pointers.num_channels,
                        "Host output width must match the prepared main output width"
                    );
                    let output_channels =
                        output_channel_pointers.num_channels.min(main_output_width);
                    for (channel_idx, output_slice) in main_slices[..main_output_width]
                        .iter_mut()
                        .enumerate()
                        .take(output_channels)
                    {
                        let output_channel_pointer =
                            output_channel_pointers.ptrs.as_ptr().add(channel_idx);

                        *output_slice = std::slice::from_raw_parts_mut(
                            (*output_channel_pointer).add(sample_offset),
                            num_samples,
                        );
                    }

                    // Missing or mismatching host output channels must not retain slices from a
                    // previous callback. The input-only suffix is rebound to scratch below.
                    main_slices[output_channels..main_output_width].fill_with(|| &mut []);
                }
                None => {
                    nih_debug_assert_eq!(main_output_width, 0);
                    main_slices[..main_output_width].fill_with(|| &mut []);
                }
            }

            for (channel_slice, channel_storage) in main_slices[main_output_width..]
                .iter_mut()
                .zip(main_input_storage.iter_mut())
            {
                nih_debug_assert!(num_samples <= channel_storage.len());
                // SAFETY: This storage belongs to the manager and remains alive for the returned
                // `Buffers` borrow. It is not accessed directly before that borrow ends.
                *channel_slice = &mut *(&mut channel_storage[..num_samples] as *mut [f32]);
            }
        });

        if let Some(input_channel_pointers) = self.main_input_channel_pointers {
            let input_channels = input_channel_pointers
                .num_channels
                .min(self.main_buffer.channels());
            nih_debug_assert!(input_channel_pointers.num_channels <= self.main_buffer.channels());
            let output_channel_pointers = self.main_output_channel_pointers;

            self.main_buffer.set_slices(num_samples, |main_slices| {
                if let Some(output_channel_pointers) = output_channel_pointers {
                    let output_channels =
                        output_channel_pointers.num_channels.min(main_output_width);
                    let copied_output_channels = input_channels.min(output_channels);

                    // Since NIH-plug processes audio in-place, input data needs to be copied to
                    // output storage unless the host already aliased that channel.
                    for (channel_idx, output_slice) in main_slices
                        .iter_mut()
                        .take(copied_output_channels)
                        .enumerate()
                    {
                        let input_channel_pointer =
                            *input_channel_pointers.ptrs.as_ptr().add(channel_idx);
                        let output_channel_pointer =
                            *output_channel_pointers.ptrs.as_ptr().add(channel_idx);

                        if input_channel_pointer != output_channel_pointer {
                            output_slice.copy_from_slice(std::slice::from_raw_parts(
                                input_channel_pointer.add(sample_offset),
                                num_samples,
                            ));
                        }
                    }

                    // Any output channels without a matching input need to be initialized so stale
                    // samples from a previous callback cannot leak through.
                    for output_slice in main_slices
                        .iter_mut()
                        .take(output_channels)
                        .skip(input_channels)
                    {
                        output_slice.fill(0.0);
                    }
                }

                // Extra main input channels have no host output pointers. Copy them into the
                // prepared scratch suffix, and clear absent channels on shorter callbacks.
                for (channel_idx, input_only_slice) in
                    main_slices.iter_mut().enumerate().skip(main_output_width)
                {
                    if channel_idx < input_channels {
                        let input_channel_pointer =
                            *input_channel_pointers.ptrs.as_ptr().add(channel_idx);
                        input_only_slice.copy_from_slice(std::slice::from_raw_parts(
                            input_channel_pointer.add(sample_offset),
                            num_samples,
                        ));
                    } else {
                        input_only_slice.fill(0.0);
                    }
                }
            });
        } else if !self.main_input_storage.is_empty() {
            // Keep existing output-backed behavior for a missing main input pointer, but never
            // expose stale samples from a prior callback through the newly added scratch suffix.
            self.main_buffer.set_slices(num_samples, |main_slices| {
                for input_only_slice in main_slices.iter_mut().skip(main_output_width) {
                    input_only_slice.fill(0.0);
                }
            });
        }

        self.main_buffer.set_main_input_channels_complete(
            self.main_input_channels == 0
                || self
                    .main_input_channel_pointers
                    .is_some_and(|pointers| pointers.num_channels >= self.main_input_channels),
        );

        // Because NIH-plug's `Buffer` type is geared around in-place processing, auxiliary inputs
        // need to be copied to our own buffers first (backed by the 'storage' vectors on this
        // object). That way the plugin can modify those buffers like any other buffers.
        for (input_channel_pointers, (input_storage, input_buffer)) in
            self.aux_input_channel_pointers.iter().zip(
                self.aux_input_storage
                    .iter_mut()
                    .zip(self.aux_input_buffers.iter_mut()),
            )
        {
            // Every channel must match this callback, including missing ports
            // and missing channels after a shorter callback. Capacity was
            // prepared for the negotiated maximum, so this does not allocate.
            for channel in input_storage.iter_mut() {
                nih_debug_assert!(num_samples <= channel.capacity());
                channel.resize(num_samples, 0.0);
            }
            // Since these buffers are backed by our own storage, we can fill them with zeroes if
            // the pointers are missing for whatever reason that might be
            match input_channel_pointers {
                Some(input_channel_pointers) => {
                    // The host may omit trailing channels from a declared auxiliary
                    // bus. The prepared storage supplies silence for those channels.
                    nih_debug_assert!(input_channel_pointers.num_channels <= input_storage.len());
                    for (channel_idx, channel) in input_storage
                        .iter_mut()
                        .enumerate()
                        .take(input_channel_pointers.num_channels)
                    {
                        let input_channel_pointer =
                            input_channel_pointers.ptrs.as_ptr().add(channel_idx);

                        channel.copy_from_slice(std::slice::from_raw_parts_mut(
                            (*input_channel_pointer).add(sample_offset),
                            num_samples,
                        ))
                    }

                    // In case we were provided too few channels we'll fill the rest with zeroes to
                    // avoid unexpected situations
                    for channel in input_storage
                        .iter_mut()
                        .skip(input_channel_pointers.num_channels)
                    {
                        channel.fill(0.0);
                    }
                }
                None => {
                    for channel in input_storage.iter_mut() {
                        channel.fill(0.0);
                    }
                }
            }

            input_buffer.set_slices(num_samples, |input_slices| {
                // Since we initialized both `input_buffer` and `input_storage` this invariant
                // should never fail unless we made an error ourselves
                debug_assert_eq!(input_slices.len(), input_storage.len());

                for (channel_slice, channel_storage) in
                    input_slices.iter_mut().zip(input_storage.iter_mut())
                {
                    // SAFETY: `channel_storage` is no longer used accessed directly after this
                    *channel_slice = &mut *(channel_storage.as_mut_slice() as *mut [f32]);
                }
            });
        }

        // The auxiliary output buffers can point directly to the host's buffers. This logic is the
        // same as the main outputs, minus the copying of input cdata
        for (output_channel_pointers, output_buffer) in self
            .aux_output_channel_pointers
            .iter()
            .zip(self.aux_output_buffers.iter_mut())
        {
            match output_channel_pointers {
                Some(output_channel_pointers) => {
                    output_buffer.set_slices(num_samples, |output_slices| {
                        nih_debug_assert_eq!(
                            output_slices.len(),
                            output_channel_pointers.num_channels
                        );
                        for (channel_idx, output_slice) in output_slices
                            .iter_mut()
                            .enumerate()
                            .take(output_channel_pointers.num_channels)
                        {
                            let output_channel_pointer =
                                output_channel_pointers.ptrs.as_ptr().add(channel_idx);

                            *output_slice = std::slice::from_raw_parts_mut(
                                (*output_channel_pointer).add(sample_offset),
                                num_samples,
                            );

                            // The host may not zero out the buffers, and assume the plugin always
                            // write something there
                            output_slice.fill(0.0);
                        }

                        // If the caller/host should have provided buffer pointers but didn't then
                        // we must get rid of any dangling slices
                        output_slices[output_channel_pointers.num_channels..].fill_with(|| &mut [])
                    });
                }
                None => {
                    output_buffer.set_slices(0, |output_slices| {
                        // Keep the declared channel slots but report no samples when the host
                        // omitted this bus. This preserves Buffer's equal-length invariant and
                        // prevents stale slices from a previously present bus.
                        output_slices.fill_with(|| &mut [])
                    });
                }
            }
        }

        // SAFETY: The 'static lifetimes on the objects are needed so we can store the buffers.
        //         Their actual lifetimes are `'a`, so we need to shrink them here. The contents are
        //         valid for as long as the returned object is borrowed.
        std::mem::transmute::<Buffers<'a, 'static>, Buffers<'a, 'buffer>>(Buffers {
            main_buffer: &mut self.main_buffer,
            aux_inputs: &mut self.aux_input_buffers,
            aux_outputs: &mut self.aux_output_buffers,
        })
    }
}

#[cfg(any(miri, test))]
mod miri {
    use super::*;
    use crate::prelude::{new_nonzero_u32, PortNames};

    const BUFFER_SIZE: usize = 512;
    const NUM_MAIN_INPUT_CHANNELS: usize = 1;
    const NUM_MAIN_OUTPUT_CHANNELS: usize = 2;

    const NUM_AUX_CHANNELS: usize = 2;
    const NUM_AUX_PORTS: usize = 2;

    const AUDIO_IO_LAYOUT: AudioIOLayout = AudioIOLayout {
        main_input_channels: Some(new_nonzero_u32(NUM_MAIN_INPUT_CHANNELS as u32)),
        main_output_channels: Some(new_nonzero_u32(NUM_MAIN_OUTPUT_CHANNELS as u32)),
        aux_input_ports: &[new_nonzero_u32(NUM_AUX_CHANNELS as u32); NUM_AUX_PORTS],
        aux_output_ports: &[new_nonzero_u32(NUM_AUX_CHANNELS as u32); NUM_AUX_PORTS],
        names: PortNames::const_default(),
    };

    #[test]
    fn buffer_io() {
        // This works very similarly to the standalone CPAL and dummy backends
        let mut main_io_storage = vec![vec![0.0f32; BUFFER_SIZE]; NUM_MAIN_OUTPUT_CHANNELS];
        let mut aux_input_storage =
            vec![vec![vec![0.0f32; BUFFER_SIZE]; NUM_AUX_CHANNELS]; NUM_AUX_PORTS];
        let mut aux_output_storage =
            vec![vec![vec![0.0f32; BUFFER_SIZE]; NUM_AUX_CHANNELS]; NUM_AUX_PORTS];

        let mut main_io_channel_pointers: Vec<*mut f32> = main_io_storage
            .iter_mut()
            .map(|channel_slice| channel_slice.as_mut_ptr())
            .collect();
        let mut aux_input_channel_pointers: Vec<Vec<*mut f32>> = aux_input_storage
            .iter_mut()
            .map(|aux_input_storage| {
                aux_input_storage
                    .iter_mut()
                    .map(|channel_slice| channel_slice.as_mut_ptr())
                    .collect()
            })
            .collect();
        let mut aux_output_channel_pointers: Vec<Vec<*mut f32>> = aux_output_storage
            .iter_mut()
            .map(|aux_output_storage| {
                aux_output_storage
                    .iter_mut()
                    .map(|channel_slice| channel_slice.as_mut_ptr())
                    .collect()
            })
            .collect();

        // The actual buffer management here works the same as in the JACK backend. See that
        // implementation for more information.
        let mut buffer_manager = BufferManager::for_audio_io_layout(BUFFER_SIZE, AUDIO_IO_LAYOUT);
        let buffers = unsafe {
            buffer_manager.create_buffers(0, BUFFER_SIZE, |buffer_sources| {
                *buffer_sources.main_output_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(main_io_channel_pointers.as_mut_ptr()).unwrap(),
                    num_channels: main_io_channel_pointers.len(),
                });
                *buffer_sources.main_input_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(main_io_channel_pointers.as_mut_ptr()).unwrap(),
                    num_channels: NUM_MAIN_INPUT_CHANNELS.min(main_io_channel_pointers.len()),
                });

                for (input_source_channel_pointers, input_channel_pointers) in buffer_sources
                    .aux_input_channel_pointers
                    .iter_mut()
                    .zip(aux_input_channel_pointers.iter_mut())
                {
                    *input_source_channel_pointers = Some(ChannelPointers {
                        ptrs: NonNull::new(input_channel_pointers.as_mut_ptr()).unwrap(),
                        num_channels: input_channel_pointers.len(),
                    });
                }

                for (output_source_channel_pointers, output_channel_pointers) in buffer_sources
                    .aux_output_channel_pointers
                    .iter_mut()
                    .zip(aux_output_channel_pointers.iter_mut())
                {
                    *output_source_channel_pointers = Some(ChannelPointers {
                        ptrs: NonNull::new(output_channel_pointers.as_mut_ptr()).unwrap(),
                        num_channels: output_channel_pointers.len(),
                    });
                }
            })
        };
        assert_eq!(buffers.main_buffer.channels(), NUM_MAIN_OUTPUT_CHANNELS);
        assert!(buffers.main_buffer.has_all_main_input_channels());

        for channel_samples in buffers
            .main_buffer
            .iter_samples()
            .chain(
                buffers
                    .aux_inputs
                    .iter_mut()
                    .flat_map(|buffer| buffer.iter_samples()),
            )
            .chain(
                buffers
                    .aux_outputs
                    .iter_mut()
                    .flat_map(|buffer| buffer.iter_samples()),
            )
        {
            for sample in channel_samples {
                *sample += 1.0;
            }
        }

        // These checks are fine due to stacked borrows even without explicitly dropping `buffers`.
        // If we were to access `buffers` again after this miri would trigger an error.
        for channel in main_io_storage
            .iter()
            .chain(aux_output_storage.iter().flat_map(|storage| storage.iter()))
        {
            for sample in channel {
                assert!(*sample == 1.0);
            }
        }

        for channel in aux_input_storage.iter().flat_map(|storage| storage.iter()) {
            for sample in channel {
                assert!(*sample == 0.0);
            }
        }
    }

    #[test]
    fn absent_auxiliary_output_reuse_keeps_sample_counts_and_slices_consistent() {
        const FRAMES: usize = 23;

        let mut main_storage = vec![vec![0.25_f32; FRAMES]; NUM_MAIN_OUTPUT_CHANNELS];
        let mut main_ptrs: Vec<*mut f32> = main_storage
            .iter_mut()
            .map(|channel| channel.as_mut_ptr())
            .collect();
        let mut aux_output_storage =
            vec![vec![vec![0.75_f32; FRAMES]; NUM_AUX_CHANNELS]; NUM_AUX_PORTS];
        let mut aux_output_ptrs: Vec<Vec<*mut f32>> = aux_output_storage
            .iter_mut()
            .map(|bus| bus.iter_mut().map(|channel| channel.as_mut_ptr()).collect())
            .collect();
        let mut manager = BufferManager::for_audio_io_layout(FRAMES, AUDIO_IO_LAYOUT);

        let assert_absent_outputs = |buffers: &Buffers<'_, '_>| {
            assert_eq!(buffers.aux_outputs.len(), NUM_AUX_PORTS);
            for output in buffers.aux_outputs.iter() {
                assert_eq!(output.channels(), NUM_AUX_CHANNELS);
                assert_eq!(output.samples(), 0);
                assert!(output
                    .as_slice_immutable()
                    .iter()
                    .all(|channel| channel.is_empty()));
            }
        };

        let buffers = unsafe {
            manager.create_buffers(0, FRAMES, |sources| {
                *sources.main_output_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(main_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: main_ptrs.len(),
                });
            })
        };
        assert_absent_outputs(&buffers);

        let buffers = unsafe {
            manager.create_buffers(0, FRAMES, |sources| {
                *sources.main_output_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(main_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: main_ptrs.len(),
                });
                sources.aux_output_channel_pointers[0] = Some(ChannelPointers {
                    ptrs: NonNull::new(aux_output_ptrs[0].as_mut_ptr()).unwrap(),
                    num_channels: NUM_AUX_CHANNELS,
                });
            })
        };
        assert_eq!(buffers.aux_outputs[0].samples(), FRAMES);
        assert_eq!(buffers.aux_outputs[0].channels(), NUM_AUX_CHANNELS);
        assert!(buffers.aux_outputs[0]
            .as_slice_immutable()
            .iter()
            .all(|channel| channel.len() == FRAMES && channel.iter().all(|sample| *sample == 0.0)));
        assert_eq!(buffers.aux_outputs[1].samples(), 0);
        assert!(buffers.aux_outputs[1]
            .as_slice_immutable()
            .iter()
            .all(|channel| channel.is_empty()));
        assert!(aux_output_storage[0]
            .iter()
            .all(|channel| channel.iter().all(|sample| *sample == 0.0)));

        let buffers = unsafe {
            manager.create_buffers(0, FRAMES, |sources| {
                *sources.main_output_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(main_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: main_ptrs.len(),
                });
            })
        };
        assert_absent_outputs(&buffers);
    }

    #[test]
    fn wide_main_input_keeps_input_only_channels_in_prepared_scratch() {
        const INPUTS: usize = 64;
        const OUTPUTS: usize = 16;
        const FRAMES: usize = 31;
        const MAX_FRAMES: usize = 257;
        const SAMPLE_OFFSET: usize = 7;
        const STORAGE_FRAMES: usize = MAX_FRAMES + SAMPLE_OFFSET;

        let layout = AudioIOLayout {
            main_input_channels: Some(new_nonzero_u32(INPUTS as u32)),
            main_output_channels: Some(new_nonzero_u32(OUTPUTS as u32)),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        };

        for aliased in [false, true] {
            let mut input_storage: Vec<Vec<f32>> = (0..INPUTS)
                .map(|channel| {
                    (0..STORAGE_FRAMES)
                        .map(|frame| channel as f32 * 10.0 + frame as f32)
                        .collect()
                })
                .collect();
            let mut output_storage = vec![vec![-1.0_f32; STORAGE_FRAMES]; OUTPUTS];
            let mut input_ptrs: Vec<*mut f32> = input_storage
                .iter_mut()
                .map(|channel| channel.as_mut_ptr())
                .collect();
            let mut output_ptrs = if aliased {
                input_ptrs[..OUTPUTS].to_vec()
            } else {
                output_storage
                    .iter_mut()
                    .map(|channel| channel.as_mut_ptr())
                    .collect()
            };
            let mut manager = BufferManager::for_audio_io_layout(MAX_FRAMES, layout);
            let prepared_scratch = manager
                .main_input_storage
                .iter()
                .map(|channel| (channel.as_ptr(), channel.len(), channel.capacity()))
                .collect::<Vec<_>>();
            let buffers = unsafe {
                manager.create_buffers(SAMPLE_OFFSET, FRAMES, |sources| {
                    *sources.main_input_channel_pointers = Some(ChannelPointers {
                        ptrs: NonNull::new(input_ptrs.as_mut_ptr()).unwrap(),
                        num_channels: INPUTS,
                    });
                    *sources.main_output_channel_pointers = Some(ChannelPointers {
                        ptrs: NonNull::new(output_ptrs.as_mut_ptr()).unwrap(),
                        num_channels: OUTPUTS,
                    });
                })
            };

            assert_eq!(buffers.main_buffer.channels(), INPUTS);
            assert!(buffers.main_buffer.has_all_main_input_channels());
            for (channel_idx, channel) in buffers.main_buffer.as_slice().iter().enumerate() {
                assert_eq!(channel.len(), FRAMES);
                for (frame, sample) in channel.iter().enumerate() {
                    assert_eq!(
                        *sample,
                        channel_idx as f32 * 10.0 + (SAMPLE_OFFSET + frame) as f32
                    );
                }
            }

            for (channel_idx, channel) in buffers.main_buffer.as_slice().iter_mut().enumerate() {
                for sample in channel.iter_mut() {
                    *sample = -(channel_idx as f32 + 1.0);
                }
            }

            for channel in 0..OUTPUTS {
                let storage = if aliased {
                    &input_storage[channel]
                } else {
                    &output_storage[channel]
                };
                assert!(storage[SAMPLE_OFFSET..SAMPLE_OFFSET + FRAMES]
                    .iter()
                    .all(|sample| *sample == -(channel as f32 + 1.0)));
                if aliased {
                    for (frame, sample) in storage.iter().enumerate().take(SAMPLE_OFFSET) {
                        assert_eq!(*sample, channel as f32 * 10.0 + frame as f32);
                    }
                    for (frame, sample) in storage.iter().enumerate().skip(SAMPLE_OFFSET + FRAMES) {
                        assert_eq!(*sample, channel as f32 * 10.0 + frame as f32);
                    }
                } else {
                    assert!(storage[..SAMPLE_OFFSET]
                        .iter()
                        .all(|sample| *sample == -1.0));
                    assert!(storage[SAMPLE_OFFSET + FRAMES..]
                        .iter()
                        .all(|sample| *sample == -1.0));
                }
            }
            for (channel, storage) in input_storage.iter().enumerate().skip(OUTPUTS) {
                assert!((0..STORAGE_FRAMES)
                    .all(|frame| { storage[frame] == channel as f32 * 10.0 + frame as f32 }));
            }
            assert_eq!(
                manager
                    .main_input_storage
                    .iter()
                    .map(|channel| (channel.as_ptr(), channel.len(), channel.capacity()))
                    .collect::<Vec<_>>(),
                prepared_scratch,
                "processing a callback must not resize the prepared input-only scratch"
            );
        }
    }

    #[test]
    fn wide_main_input_scratch_handles_max_short_and_missing_callbacks() {
        const INPUTS: usize = 64;
        const OUTPUTS: usize = 16;
        const MAX_FRAMES: usize = 257;
        const FULL_OFFSET: usize = 3;
        const SHORT_INPUTS: usize = 18;
        const SHORT_FRAMES: usize = 5;

        let layout = AudioIOLayout {
            main_input_channels: Some(new_nonzero_u32(INPUTS as u32)),
            main_output_channels: Some(new_nonzero_u32(OUTPUTS as u32)),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        };
        let mut input_storage: Vec<Vec<f32>> = (0..INPUTS)
            .map(|channel| {
                (0..MAX_FRAMES + FULL_OFFSET)
                    .map(|frame| channel as f32 * 1000.0 + frame as f32 + 1.0)
                    .collect()
            })
            .collect();
        let mut output_storage = vec![vec![-1.0_f32; MAX_FRAMES + FULL_OFFSET]; OUTPUTS];
        let mut input_ptrs: Vec<*mut f32> = input_storage
            .iter_mut()
            .map(|channel| channel.as_mut_ptr())
            .collect();
        let mut output_ptrs: Vec<*mut f32> = output_storage
            .iter_mut()
            .map(|channel| channel.as_mut_ptr())
            .collect();
        let mut manager = BufferManager::for_audio_io_layout(MAX_FRAMES, layout);
        let prepared_scratch = manager
            .main_input_storage
            .iter()
            .map(|channel| (channel.as_ptr(), channel.len(), channel.capacity()))
            .collect::<Vec<_>>();

        // The first callback uses the full negotiated maximum and a nonzero source offset.
        let buffers = unsafe {
            manager.create_buffers(FULL_OFFSET, MAX_FRAMES, |sources| {
                *sources.main_input_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(input_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: INPUTS,
                });
                *sources.main_output_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(output_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: OUTPUTS,
                });
            })
        };
        assert_eq!(buffers.main_buffer.channels(), INPUTS);
        assert!(buffers.main_buffer.has_all_main_input_channels());
        for (channel_idx, channel) in buffers.main_buffer.as_slice().iter().enumerate() {
            assert_eq!(channel.len(), MAX_FRAMES);
            for (frame, sample) in channel.iter().enumerate() {
                assert_eq!(
                    *sample,
                    channel_idx as f32 * 1000.0 + (FULL_OFFSET + frame) as f32 + 1.0
                );
            }
        }
        for channel in 0..OUTPUTS {
            assert_eq!(
                &output_storage[channel][FULL_OFFSET..FULL_OFFSET + MAX_FRAMES],
                &input_storage[channel][FULL_OFFSET..FULL_OFFSET + MAX_FRAMES]
            );
            assert!(output_storage[channel][..FULL_OFFSET]
                .iter()
                .all(|sample| *sample == -1.0));
            assert!(output_storage[channel][FULL_OFFSET + MAX_FRAMES..]
                .iter()
                .all(|sample| *sample == -1.0));
        }

        // A shorter callback with fewer input channels must clear absent scratch channels instead
        // of exposing the previous full-width callback's samples.
        let buffers = unsafe {
            manager.create_buffers(0, SHORT_FRAMES, |sources| {
                *sources.main_input_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(input_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: SHORT_INPUTS,
                });
                *sources.main_output_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(output_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: OUTPUTS,
                });
            })
        };
        assert_eq!(buffers.main_buffer.channels(), INPUTS);
        assert!(!buffers.main_buffer.has_all_main_input_channels());
        for (channel_idx, channel) in buffers.main_buffer.as_slice().iter().enumerate() {
            assert_eq!(channel.len(), SHORT_FRAMES);
            for (frame, sample) in channel.iter().enumerate() {
                let expected = if channel_idx < SHORT_INPUTS {
                    channel_idx as f32 * 1000.0 + frame as f32 + 1.0
                } else {
                    0.0
                };
                assert_eq!(*sample, expected);
            }
        }

        // A missing primary input leaves the legacy output-backed prefix behavior alone, but the
        // scratch suffix is cleared on every callback. The native Ambisonics wrapper rejects a
        // missing primary input before invoking DSP.
        let buffers = unsafe {
            manager.create_buffers(0, 3, |sources| {
                *sources.main_output_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(output_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: OUTPUTS,
                });
            })
        };
        assert_eq!(buffers.main_buffer.channels(), INPUTS);
        assert!(!buffers.main_buffer.has_all_main_input_channels());
        for input_only_slice in buffers.main_buffer.as_slice().iter().skip(OUTPUTS) {
            assert!(input_only_slice.iter().all(|sample| *sample == 0.0));
        }

        assert_eq!(
            manager
                .main_input_storage
                .iter()
                .map(|channel| (channel.as_ptr(), channel.len(), channel.capacity()))
                .collect::<Vec<_>>(),
            prepared_scratch,
            "varying callback shapes must reuse the prepared input-only scratch"
        );
    }

    #[test]
    fn input_only_layout_is_available_without_a_main_output_bus() {
        const INPUTS: usize = 3;
        const FRAMES: usize = 11;
        let layout = AudioIOLayout {
            main_input_channels: Some(new_nonzero_u32(INPUTS as u32)),
            main_output_channels: None,
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        };
        let mut input_storage = vec![vec![0.0_f32; FRAMES]; INPUTS];
        for (channel_idx, channel) in input_storage.iter_mut().enumerate() {
            for (frame, sample) in channel.iter_mut().enumerate() {
                *sample = channel_idx as f32 * 100.0 + frame as f32;
            }
        }
        let mut input_ptrs: Vec<*mut f32> = input_storage
            .iter_mut()
            .map(|channel| channel.as_mut_ptr())
            .collect();
        let mut manager = BufferManager::for_audio_io_layout(FRAMES, layout);
        let buffers = unsafe {
            manager.create_buffers(0, FRAMES, |sources| {
                *sources.main_input_channel_pointers = Some(ChannelPointers {
                    ptrs: NonNull::new(input_ptrs.as_mut_ptr()).unwrap(),
                    num_channels: INPUTS,
                });
            })
        };
        assert_eq!(buffers.main_buffer.channels(), INPUTS);
        assert!(buffers.main_buffer.has_all_main_input_channels());
        for (channel_idx, channel) in buffers.main_buffer.as_slice().iter().enumerate() {
            for (frame, sample) in channel.iter().enumerate() {
                assert_eq!(*sample, channel_idx as f32 * 100.0 + frame as f32);
            }
        }
    }
}
