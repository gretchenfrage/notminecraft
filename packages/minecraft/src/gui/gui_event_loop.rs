
use crate::{
	asset::Assets,
	sound::SoundPlayer,
	thread_pool::ThreadPool,
	game_data::GameData,
	settings::{Settings, SETTINGS_FILE_NAME},
	gui::{
		context::{
			GuiGlobalContext,
			GuiSpatialContext,
			GuiWindowContext,
			FocusLevel,
		},
		event::{
			ScrolledAmount,
			TypingInput,
			TypingControl,
		},
		state_frame::GuiStateFrame,
		state_frame_obj::GuiStateFrameObj,
		fps_overlay::FpsOverlay,
		clipboard::Clipboard,
	},
};
use get_assets::DataDir;
use graphics::{
	Renderer,
	frame_content::FrameContent,
};
use std::{
	collections::{
		HashSet,
		VecDeque,
	},
	sync::Arc,
	cell::RefCell,
	time::{
		UNIX_EPOCH,
		Instant,
		Duration,
		SystemTime,
	},
	panic::{
		catch_unwind,
		AssertUnwindSafe,
	},
	process::exit,
	env,
};
use winit::{
    event_loop::{
    	ControlFlow,
    	EventLoop,
    },
    window::{
    	Window,
    	WindowBuilder,
    	CursorGrabMode,
    },
    event::{
    	Event,
    	WindowEvent,
    	DeviceEvent,
	    MouseButton,
	    ElementState,
	    MouseScrollDelta,
	    StartCause,
	},
	dpi::{
		LogicalSize,
		PhysicalSize,
		PhysicalPosition,
	},
	keyboard::PhysicalKey,
};
use tokio::runtime::Handle;
use pollster::FutureExt;
use vek::*;


#[derive(Debug)]
pub struct EventLoopEffectQueue(VecDeque<EventLoopEffect>);

#[derive(Debug)]
enum EventLoopEffect {
	PopStateFrame,
	PushStateFrame(Box<dyn GuiStateFrameObj>),
	SetScale(f32),
	CaptureMouse,
	UncaptureMouse,
}

impl EventLoopEffectQueue {
	pub fn pop_state_frame(&mut self) {
		self.0.push_back(EventLoopEffect::PopStateFrame);
	}

	pub fn push_state_frame<T>(&mut self, state_frame: T)
	where
		T: GuiStateFrame + 'static,
	{
		self.push_state_frame_obj(Box::new(state_frame))
	}

	pub fn push_state_frame_obj(
		&mut self,
		state_frame: Box<dyn GuiStateFrameObj>,
	) {
		self.0.push_back(EventLoopEffect::PushStateFrame(state_frame));
	}

	pub fn set_scale(&mut self, scale: f32) {
		self.0.push_back(EventLoopEffect::SetScale(scale));
	}

	pub fn capture_mouse(&mut self) {
		self.0.push_back(EventLoopEffect::CaptureMouse);
	}

	pub fn uncapture_mouse(&mut self) {
		self.0.push_back(EventLoopEffect::UncaptureMouse);
	}
}


struct State {
	effect_queue: RefCell<EventLoopEffectQueue>,

	calibration_instant: Instant,
	calibration_time_since_epoch: Duration,

    renderer: RefCell<Renderer>,
    frame_duration_target: Duration,
    next_frame_target: Instant,
    tokio: Handle,
    clipboard: Clipboard,
    thread_pool: ThreadPool,
    sound_player: SoundPlayer,
    assets: Assets,
    data_dir: DataDir,
    settings: RefCell<Settings>,
    game: Arc<GameData>,
    focus_level: FocusLevel,
	pressed_keys: HashSet<PhysicalKey>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    cursor_pos: Option<Vec2<f32>>,
    size: Extent2<u32>,
    os_scale: f32,
    app_scale: f32,

    // TODO: only necessary until they stabilize const set constructor
    pressed_keys_empty: HashSet<PhysicalKey>,
    pressed_mouse_buttons_empty: HashSet<MouseButton>,

}

impl State {
	fn new(
		window: &Window,
		frame_duration_target: Duration,
		renderer: Renderer,
		tokio: Handle,
		thread_pool: ThreadPool,
		sound_player: SoundPlayer,
		assets: Assets,
		data_dir: DataDir,
		game: Arc<GameData>,
	) -> Self
	{
		// the "calibration" part is just that we try to capture these as close
		// to simultaneously as we can achieve
		let calibration_instant = Instant::now();
		let calibration_system_time = SystemTime::now();

		let winit_size = window.inner_size();
		State {
			effect_queue: RefCell::new(EventLoopEffectQueue(VecDeque::new())),
			calibration_instant,
			calibration_time_since_epoch: calibration_system_time
				.duration_since(UNIX_EPOCH)
				.unwrap_or_else(|_| {
					warn!("system time is before unix epoch");
					Duration::ZERO
				}),
			renderer: RefCell::new(renderer),
			frame_duration_target,
			// random default value, hopefully never gets used
			next_frame_target: Instant::now(),
			tokio,
			clipboard: Clipboard::new(),
			thread_pool,
			sound_player,
			assets,
			settings: RefCell::new(Settings::read(data_dir.subdir(SETTINGS_FILE_NAME))),
			data_dir,
			game,
			focus_level: FocusLevel::Focused,
			pressed_keys: HashSet::new(),
			pressed_mouse_buttons: HashSet::new(),
			cursor_pos: None,
			size: Extent2 {
				w: winit_size.width,
				h: winit_size.height,
			},
			os_scale: window.scale_factor() as f32,
			app_scale: 1.0,

			pressed_keys_empty: HashSet::new(),
			pressed_mouse_buttons_empty: HashSet::new(),
		}
	}

	fn with_ctx<F>(&self, f: F)
	where
		F: FnOnce(&GuiWindowContext),
	{
		f(&GuiWindowContext {
			spatial: GuiSpatialContext {
				global: &GuiGlobalContext {
					event_loop: &self.effect_queue,
					time_since_epoch: Instant::now() - self.calibration_instant + self.calibration_time_since_epoch,
					renderer: &self.renderer,
					frame_duration_target: self.frame_duration_target,
					next_frame_target: self.next_frame_target,
					tokio: &self.tokio,
					clipboard: &self.clipboard,
					thread_pool: &self.thread_pool,
					sound_player: &self.sound_player,
					assets: &self.assets,
					data_dir: &self.data_dir,
					settings: &self.settings,
					game: &self.game,
					focus_level: self.focus_level,
					pressed_keys:
						if self.focus_level >= FocusLevel::Focused {
							&self.pressed_keys
						} else { &self.pressed_keys_empty },
					pressed_mouse_buttons: 
						if self.focus_level >= FocusLevel::Focused {
							&self.pressed_mouse_buttons
						} else { &self.pressed_mouse_buttons_empty },
				},
				cursor_pos: self.cursor_pos
					.filter(|_| self.focus_level < FocusLevel::MouseCaptured),
			},
			size: self.size,
			scale: self.os_scale * self.app_scale,
		})
	}
}


struct Stack(Vec<Box<dyn GuiStateFrameObj>>);

impl Stack {
	fn new(state_frame: Box<dyn GuiStateFrameObj>) -> Self {
		Stack(vec![state_frame])
	}

	fn top(&mut self) -> &mut dyn GuiStateFrameObj {
		&mut **self.0.iter_mut().rev().next().unwrap()
	}
}


pub struct GuiEventLoop {
	event_loop: EventLoop<()>,
	window: Arc<Window>,
	pub renderer: Renderer,
	tokio: Handle,
	thread_pool: ThreadPool,
	sound_player: SoundPlayer,
}

impl GuiEventLoop {
	pub fn new(tokio: &Handle, thread_pool: ThreadPool) -> Self {
		let event_loop = EventLoop::new()
			.expect("failed to create event loop");
		let window = WindowBuilder::new()
			.with_inner_size(LogicalSize::new(854, 480))
			.with_title("Not Minecraft")
			.build(&event_loop)
			.expect("failed to build window");
		let window = Arc::new(window);
	
		let renderer = Renderer::new(Arc::clone(&window))
			.block_on()
			.expect("failed to create renderer");
		let sound_player = SoundPlayer::new()
			.expect("failed to create sound player");
		
		GuiEventLoop {
			event_loop,
			window,
			renderer,
			tokio: Handle::clone(&tokio),
			thread_pool,
			sound_player,
		}		
	}

	pub fn run(
		self,
		state_frame: Box<dyn GuiStateFrameObj>,
		assets: Assets,
		data_dir: DataDir,
		game: Arc<GameData>,
	) -> ! {
		// decide what FPS to try and render at
		const MIN_AUTO_MILLIHERTZ: u32 = 1000 * 60;
		let millihertz = self.event_loop.available_monitors()
			.filter_map(|monitor| monitor.refresh_rate_millihertz())
			.filter(|&millihertz| millihertz >= MIN_AUTO_MILLIHERTZ)
			.max()
			.unwrap_or(MIN_AUTO_MILLIHERTZ);

		let mut stack = Stack::new(state_frame);
		let mut state = State::new(
			&self.window,
			Duration::from_secs(1000) / millihertz,
			self.renderer,
			self.tokio,
			self.thread_pool,
			self.sound_player,
			assets,
			data_dir,
			game,
		);

		let mut prev_update_time = None;
		let mut fps_queue = VecDeque::new();

		let mut frame_is_happening: bool = true;
		
		let result = self.event_loop.run(move |event, target| {
			trace!(?event, "winit event");

			if target.exiting() {
				return;
			}

			match event {
				Event::NewEvents(cause) => {
					let now = Instant::now();
					frame_is_happening = match cause {
						StartCause::ResumeTimeReached { .. } => true,
						StartCause::WaitCancelled { .. } => false,
						StartCause::Poll => panic!("event loop unexpectedly entered polling mode"),
						StartCause::Init => true,
					};
					if frame_is_happening {
						state.next_frame_target = now + state.frame_duration_target;
						target.set_control_flow(ControlFlow::WaitUntil(state.next_frame_target));
					}
				}
				Event::WindowEvent { event, .. } => match event {
					WindowEvent::Resized(winit_size) => {
						state.size.w = winit_size.width;
						state.size.h = winit_size.height;
					}
					WindowEvent::CloseRequested => {
						stack.0.clear();
						target.exit();
					}
					WindowEvent::Destroyed => {
						stack.0.clear();
						target.exit();
					}
					WindowEvent::Focused(focused) => {
						if state.focus_level == FocusLevel::MouseCaptured {
							try_uncapture_mouse(&self.window);
						}

						state.focus_level =
							if focused { FocusLevel::Focused }
							else { FocusLevel::Unfocused };
						state.with_ctx(|ctx| stack.top().on_focus_change(ctx));
					}
					WindowEvent::KeyboardInput {
						is_synthetic: false,
						event,
						..
					} => {
						let focused = state.focus_level >= FocusLevel::Focused;
						match event.state {
							ElementState::Pressed => {
								let changed = state
									.pressed_keys
									.insert(event.physical_key);
								if changed && focused {
									let typing = event.text.as_ref()
										.map(|s| s.as_str())
										.and_then(|s| {
											let c = s.chars().next().unwrap();
											if c.is_control() {
												match c {
													'\u{8}' => Some(TypingInput::Control(TypingControl::Backspace)),
													'\r' => Some(TypingInput::Control(TypingControl::Enter)),
													'\t' => Some(TypingInput::Control(TypingControl::Tab)),
													'\u{7f}' => Some(TypingInput::Control(TypingControl::Delete)),
													c => {
														debug!(?c, "ignoring unknown control character");
														None
													},
												}
											} else {
												Some(TypingInput::Text(s))
											}
										});
									state.with_ctx(|ctx| stack
										.top()
										.on_key_press(ctx, event.physical_key, typing));
								}
							}
							ElementState::Released => {
								let changed = state
									.pressed_keys
									.remove(&event.physical_key);
								if changed && focused {
									state.with_ctx(|ctx| stack
										.top()
										.on_key_release(ctx, event.physical_key));
								}
							}
						}
					}
					WindowEvent::CursorMoved { position, .. } => {
						state.cursor_pos = Some(Vec2 {
							x: position.x as f32,
							y: position.y as f32,
						});
						if state.focus_level < FocusLevel::MouseCaptured {
							state.with_ctx(|ctx| stack.top().on_cursor_move(ctx));
						}
					}
					WindowEvent::MouseWheel { delta, .. } => {
						let amount = match delta {
							MouseScrollDelta::LineDelta(
								x,
								y,
							) => ScrolledAmount::Lines(Vec2 { x, y }),
							MouseScrollDelta::PixelDelta(
								pos,
							) => ScrolledAmount::Pixels(Vec2 {
								x: pos.x as f32,
								y: pos.y as f32,
							}),
						};
						match state.focus_level {
							FocusLevel::Unfocused => (),
							FocusLevel::Focused => {
								state.with_ctx(|ctx| stack
									.top()
									.on_cursor_scroll(ctx, amount));
							}
							FocusLevel::MouseCaptured => {
								state.with_ctx(|ctx| stack
									.top()
									.on_captured_mouse_scroll(ctx, amount));
							}
						}
					}
					WindowEvent::MouseInput {
						state: element_state,
						button,
						..
					} => match element_state {
						ElementState::Pressed => {
							let changed = state
								.pressed_mouse_buttons
								.insert(button);
							if changed {
								match state.focus_level {
									FocusLevel::Unfocused => (),
									FocusLevel::Focused => {
										state.with_ctx(|ctx| stack
											.top()
											.on_cursor_click(ctx, button));
									}
									FocusLevel::MouseCaptured => {
										state.with_ctx(|ctx| stack
											.top()
											.on_captured_mouse_click(ctx, button));
									}
								}
							}
						}
						ElementState::Released => {
							let changed = state
								.pressed_mouse_buttons
								.remove(&button);
							if changed {
								match state.focus_level {
									FocusLevel::Unfocused => (),
									FocusLevel::Focused => {
										state.with_ctx(|ctx| stack
											.top()
											.on_cursor_release(ctx, button));
									}
									FocusLevel::MouseCaptured => {
										state.with_ctx(|ctx| stack
											.top()
											.on_captured_mouse_release(
												ctx,
												button,
											));
									}
								}
							}
						}
					}
					WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
						state.os_scale = scale_factor as f32;
					}
					_ => (),
				}
				Event::DeviceEvent { event, .. } => match event {
					DeviceEvent::MouseMotion { delta: (x, y) } => {
						if state.focus_level == FocusLevel::MouseCaptured {
							state.with_ctx(|ctx| stack
								.top()
								.on_captured_mouse_move(
									ctx,
									Vec2 { x: x as f32, y: y as f32 },
								));
						}
					}
					_ => (),
				}
				Event::AboutToWait => if frame_is_happening {
					state.with_ctx(|ctx| {
						// TODO: kinda awkward to just have this right here
						let big = ctx.size.w >= 960 && ctx.size.h >= 720;
				        if big {
				            ctx.global().set_scale(2.0);
				        } else {
				            ctx.global().set_scale(1.0);
				        }

						let curr_update_time = Instant::now();

						if let Some(prev_update_time) = prev_update_time {
							let elapsed: Duration = curr_update_time - prev_update_time;
							let elapsed = elapsed.as_secs_f32();

							// TODO: tacked on
							let update_result =
								catch_unwind(AssertUnwindSafe(||
									stack.top().update(ctx, elapsed)
								));
							if update_result.is_err() {
								stack.0.pop().unwrap();

								state.effect_queue.borrow_mut().uncapture_mouse();

								if stack.0.is_empty() {
									stack.0.clear();
									target.exit();
									return;
								}
							}
						}

						prev_update_time = Some(curr_update_time);

						fps_queue.push_back(curr_update_time);

						while fps_queue
							.front()
							.map(|&update_time|
								curr_update_time - update_time
								> Duration::from_secs(1)
							)
							.unwrap_or(false)
						{
							fps_queue.pop_front().unwrap();
						}
						let fps = fps_queue.len();
						//info!(%fps);

						let mut frame_content = FrameContent::new();
						stack.top().draw(ctx, &mut frame_content);

						let mut fps_overlay = FpsOverlay::new(fps as f32, ctx.assets());
						fps_overlay.draw(ctx, &mut frame_content);
						
						if state.renderer.borrow().size() != state.size {
							state.renderer.borrow_mut().resize(state.size);
						}

						ctx.spatial.global.renderer
							.borrow_mut()
							.draw_frame(&frame_content)
							.expect("failed to draw frame");
					});
				},
				Event::LoopExiting => {
					stack.0.clear();
					target.exit();
				}
				_ => (),
			}

			let mut effect_queue = state.effect_queue.borrow_mut();
			while let Some(effect) = effect_queue.0.pop_front() {
				match effect {
					EventLoopEffect::PopStateFrame => {
						stack.0.pop().unwrap();

						effect_queue.uncapture_mouse();

						if stack.0.is_empty() {
							stack.0.clear();stack.0.clear();
							target.exit();
							return;
						}
					}
					EventLoopEffect::PushStateFrame(state_frame) => {
						stack.0.push(state_frame);
					}
					EventLoopEffect::SetScale(scale) => {
						//state.app_scale = scale / state.os_scale;
						state.app_scale = scale;
					}
					EventLoopEffect::CaptureMouse => {
						if state.focus_level < FocusLevel::MouseCaptured {
							if try_capture_mouse(&self.window) {
								state.focus_level = FocusLevel::MouseCaptured;
							}
						}
					}
					EventLoopEffect::UncaptureMouse => {
						if state.focus_level == FocusLevel::MouseCaptured {
							try_center_cursor(&self.window);
							state.focus_level = FocusLevel::Focused;
						}
						try_uncapture_mouse(&self.window);
					}
				}
			}
		});
		error!(?result, "event loop exited");
		drop(result);
		exit(0);
	}
}

fn try_center_cursor(window: &Window) {
	let PhysicalSize { width, height } = window.outer_size();
	let center = PhysicalPosition::new(width / 2, height / 2);

	match window.set_cursor_position(center) {
		Ok(()) => (),
		Err(e) => {
			error!("error centering cursor: {}", e);
		}
	}
}

fn try_capture_mouse(window: &Window) -> bool {
	if env::var("NO_CAPTURE_MOUSE").map(|s| !s.is_empty()).unwrap_or(false) {
		trace!("not capturing mouse (disabled by env var)");
		return true;
	}
	let success =
		[
			CursorGrabMode::Locked,
			CursorGrabMode::Confined,
		]
		.into_iter()
		.enumerate()
		.find(|&(i, mode)| match window.set_cursor_grab(mode) {
			Ok(()) => {
				if i > 0 {
					trace!(
						"success on fallback Window::set_cursor_grab({:?})",
						mode,
					);
				}
				window.set_cursor_visible(false);
				true
			}
			Err(e) => {
				trace!("error on Window::set_cursor_grab({:?}): {}", mode, e);
				false
			}
		})
		.is_some();
	if !success {
		error!("failed to capture mouse");
	}
	success
}

fn try_uncapture_mouse(window: &Window) {
	window.set_cursor_visible(true);
	match window.set_cursor_grab(CursorGrabMode::None) {
		Ok(()) => (),
		Err(e) => {
			error!("error on Window::set_cursor_grab(None): {}", e);
		}
	}
}
