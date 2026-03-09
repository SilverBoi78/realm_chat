use std::sync::{Arc, Mutex};

use egui::{Color32, FontId, RichText, ScrollArea};
use uuid::Uuid;

use common::{ChatMessage, models::FriendRequest, protocol::FriendEntry};

use crate::{
    api,
    state::{AuthState, ChatState, FriendsState, MainView, Screen},
    ws::WsClient,
};

const ACCENT: Color32 = Color32::from_rgb(88, 101, 242);
const BG_DARK: Color32 = Color32::from_rgb(30, 31, 34);
const BG_MID: Color32 = Color32::from_rgb(43, 45, 49);
const BG_LIGHT: Color32 = Color32::from_rgb(54, 57, 63);
const TEXT_MUTED: Color32 = Color32::from_rgb(148, 155, 164);

pub struct RealmChatApp {
    screen: Screen,
    auth: AuthState,
    chat: ChatState,
    friends: FriendsState,
    main_view: MainView,
    show_friends_sidebar: bool,
    ws: Option<WsClient>,
    rt: tokio::runtime::Handle,
    pending: Arc<Mutex<Vec<PendingResult>>>,
}

enum PendingResult {
    AuthSuccess { token: String, user_id: Uuid, username: String },
    AuthError(String),
    WorldsLoaded(Vec<common::models::World>),
    LocationsLoaded(Vec<common::models::Location>),
    MessagesLoaded(Vec<ChatMessage>),
    WorldCreated(common::models::World),
    LocationCreated(common::models::Location),
    FriendsLoaded(common::protocol::FriendsResponse),
    DmHistoryLoaded(Vec<common::models::DirectMessage>),
    FriendRequestSent(FriendRequest),
    FriendAccepted(FriendRequest),
    FriendRemoved(Uuid),
    ApiError(String),
    FriendApiError(String),
}

impl RealmChatApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, rt: tokio::runtime::Handle) -> Self {
        Self {
            screen: Screen::Login,
            auth: AuthState::default(),
            chat: ChatState { new_world_theme: "fantasy".into(), ..Default::default() },
            friends: FriendsState::default(),
            main_view: MainView::default(),
            show_friends_sidebar: false,
            ws: None,
            rt,
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn do_login(&self) {
        let u = self.auth.login_username.clone();
        let p = self.auth.login_password.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::login(&u, &p).await {
                Ok(r) => PendingResult::AuthSuccess { token: r.token, user_id: r.user_id, username: r.username },
                Err(e) => PendingResult::AuthError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn do_register(&self) {
        let u = self.auth.register_username.clone();
        let p = self.auth.register_password.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::register(&u, &p).await {
                Ok(r) => PendingResult::AuthSuccess { token: r.token, user_id: r.user_id, username: r.username },
                Err(e) => PendingResult::AuthError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn load_worlds(&self) {
        let token = self.auth.token.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::list_worlds(&token).await {
                Ok(w) => PendingResult::WorldsLoaded(w),
                Err(e) => PendingResult::ApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn load_locations(&self, world_id: Uuid) {
        let token = self.auth.token.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::list_locations(&token, world_id).await {
                Ok(l) => PendingResult::LocationsLoaded(l),
                Err(e) => PendingResult::ApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn load_messages(&self, world_id: Uuid, loc_id: Uuid) {
        let token = self.auth.token.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::get_messages(&token, world_id, loc_id).await {
                Ok(m) => PendingResult::MessagesLoaded(m),
                Err(e) => PendingResult::ApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn load_friends(&self) {
        let token = self.auth.token.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::list_friends(&token).await {
                Ok(r) => PendingResult::FriendsLoaded(r),
                Err(e) => PendingResult::ApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn load_dm_history(&self, peer_id: Uuid) {
        let token = self.auth.token.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::get_dm_history(&token, peer_id).await {
                Ok(msgs) => PendingResult::DmHistoryLoaded(msgs),
                Err(e) => PendingResult::ApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn do_send_friend_request(&self) {
        let token = self.auth.token.clone();
        let username = self.friends.add_friend_input.trim().to_owned();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::send_friend_request(&token, &username).await {
                Ok(r) => PendingResult::FriendRequestSent(r),
                Err(e) => PendingResult::FriendApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn do_accept_friend_request(&self, friendship_id: Uuid) {
        let token = self.auth.token.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::accept_friend_request(&token, friendship_id).await {
                Ok(r) => PendingResult::FriendAccepted(r),
                Err(e) => PendingResult::ApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn do_remove_friend(&self, friendship_id: Uuid) {
        let token = self.auth.token.clone();
        let pending = Arc::clone(&self.pending);
        self.rt.spawn(async move {
            let result = match api::remove_friend(&token, friendship_id).await {
                Ok(()) => PendingResult::FriendRemoved(friendship_id),
                Err(e) => PendingResult::ApiError(e.to_string()),
            };
            pending.lock().unwrap().push(result);
        });
    }

    fn open_dm(&mut self, peer_id: Uuid) {
        self.friends.active_dm_peer = Some(peer_id);
        self.friends.dm_messages.clear();
        self.friends.dm_compose.clear();
        self.main_view = MainView::DirectMessage;
        self.chat.selected_location = None;
        self.load_dm_history(peer_id);
    }

    fn process_pending(&mut self, ctx: &egui::Context) {
        let results: Vec<PendingResult> = {
            let mut lock = self.pending.lock().unwrap();
            std::mem::take(&mut *lock)
        };
        for result in results {
            match result {
                PendingResult::AuthSuccess { token, user_id, username } => {
                    self.auth.token = token.clone();
                    self.auth.user_id = Some(user_id);
                    self.auth.username = username;
                    self.auth.error = None;
                    self.ws = Some(WsClient::spawn(token, &self.rt));
                    self.screen = Screen::Main;
                    self.load_worlds();
                    self.load_friends();
                }
                PendingResult::AuthError(e) => {
                    self.auth.error = Some(e);
                }
                PendingResult::WorldsLoaded(worlds) => {
                    self.chat.worlds = worlds;
                }
                PendingResult::LocationsLoaded(locs) => {
                    self.chat.locations = locs;
                    self.chat.selected_location = None;
                    self.chat.messages.clear();
                }
                PendingResult::MessagesLoaded(msgs) => {
                    self.chat.messages = msgs;
                }
                PendingResult::WorldCreated(w) => {
                    let id = w.id;
                    self.chat.worlds.push(w);
                    self.chat.selected_world = Some(id);
                    self.chat.show_create_world = false;
                    self.load_locations(id);
                }
                PendingResult::LocationCreated(l) => {
                    let id = l.id;
                    self.chat.locations.push(l);
                    self.chat.show_create_location = false;
                    if let Some(wid) = self.chat.selected_world {
                        if let Some(ws) = &self.ws {
                            ws.join(id);
                        }
                        self.chat.selected_location = Some(id);
                        self.main_view = MainView::LocationChat;
                        self.load_messages(wid, id);
                    }
                }
                PendingResult::FriendsLoaded(resp) => {
                    self.friends.friends = resp.friends;
                    self.friends.pending_incoming = resp.pending_incoming;
                    self.friends.pending_outgoing = resp.pending_outgoing;
                }
                PendingResult::DmHistoryLoaded(msgs) => {
                    self.friends.dm_messages = msgs;
                }
                PendingResult::FriendRequestSent(req) => {
                    self.friends.pending_outgoing.push(req);
                    self.friends.add_friend_error = None;
                    self.friends.add_friend_input.clear();
                }
                PendingResult::FriendAccepted(req) => {
                    self.friends.pending_incoming.retain(|r| r.id != req.id);
                    let (peer_id, peer_name) = if Some(req.requester_id) == self.auth.user_id {
                        (req.addressee_id, req.addressee_name)
                    } else {
                        (req.requester_id, req.requester_name)
                    };
                    self.friends.friends.push(FriendEntry { user_id: peer_id, username: peer_name });
                }
                PendingResult::FriendRemoved(friendship_id) => {
                    self.friends.pending_incoming.retain(|r| r.id != friendship_id);
                    self.friends.pending_outgoing.retain(|r| r.id != friendship_id);
                    if let Some(peer_id) = self.friends.active_dm_peer {
                        if self.friends.friends.iter().any(|f| f.user_id == peer_id) {
                            if let Some(idx) = self.friends.friends.iter().position(|f| f.user_id == peer_id) {
                                if self.friends.pending_incoming.iter().chain(self.friends.pending_outgoing.iter()).any(|r| r.id == friendship_id) {
                                    self.friends.friends.remove(idx);
                                }
                            }
                        }
                    }
                    self.friends.friends.retain(|_| true);
                    self.load_friends();
                }
                PendingResult::FriendApiError(e) => {
                    self.friends.add_friend_error = Some(e);
                }
                PendingResult::ApiError(e) => {
                    tracing::error!("API error: {e}");
                }
            }
            ctx.request_repaint();
        }

        if let Some(ws) = &self.ws {
            let new_msgs = ws.drain_messages();
            if !new_msgs.is_empty() {
                let selected_loc = self.chat.selected_location;
                for msg in new_msgs {
                    if Some(msg.location_id) == selected_loc {
                        self.chat.messages.push(msg);
                    }
                }
                ctx.request_repaint();
            }

            let new_dms = ws.drain_dms();
            if !new_dms.is_empty() {
                for dm in new_dms {
                    if self.friends.active_dm_peer == Some(dm.sender_id)
                        || self.friends.active_dm_peer == Some(dm.receiver_id)
                    {
                        self.friends.dm_messages.push(dm);
                    }
                }
                ctx.request_repaint();
            }

            let new_frs = ws.drain_friend_requests();
            if !new_frs.is_empty() {
                for req in new_frs {
                    if req.status == common::models::FriendStatus::Accepted {
                        self.friends.pending_outgoing.retain(|r| r.id != req.id);
                        let (peer_id, peer_name) = if Some(req.requester_id) == self.auth.user_id {
                            (req.addressee_id, req.addressee_name.clone())
                        } else {
                            (req.requester_id, req.requester_name.clone())
                        };
                        if !self.friends.friends.iter().any(|f| f.user_id == peer_id) {
                            self.friends.friends.push(FriendEntry { user_id: peer_id, username: peer_name });
                        }
                    } else {
                        self.friends.unread_requests += 1;
                        self.friends.pending_incoming.push(req);
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    fn ui_login(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(BG_DARK))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(120.0);
                    ui.label(RichText::new("RealmChat").font(FontId::proportional(36.0)).color(ACCENT));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Chat. Quest. Conquer.").color(TEXT_MUTED));
                    ui.add_space(40.0);

                    egui::Frame::default()
                        .fill(BG_MID)
                        .rounding(8.0)
                        .inner_margin(24.0)
                        .show(ui, |ui| {
                            ui.set_width(360.0);
                            ui.heading(if self.screen == Screen::Login { "Sign In" } else { "Create Account" });
                            ui.add_space(16.0);

                            let username = if self.screen == Screen::Login {
                                &mut self.auth.login_username
                            } else {
                                &mut self.auth.register_username
                            };
                            ui.label("Username");
                            let w = ui.available_width();
                            let user_resp = ui.add(
                                egui::TextEdit::singleline(username)
                                    .desired_width(w)
                                    .hint_text("Enter username"),
                            );

                            ui.add_space(8.0);
                            let password = if self.screen == Screen::Login {
                                &mut self.auth.login_password
                            } else {
                                &mut self.auth.register_password
                            };
                            ui.label("Password");
                            let w = ui.available_width();
                            let pass_resp = ui.add(
                                egui::TextEdit::singleline(password)
                                    .password(true)
                                    .desired_width(w)
                                    .hint_text("Enter password"),
                            );

                            ui.add_space(16.0);
                            let w = ui.available_width();
                            let submit = ui.add_sized(
                                [w, 36.0],
                                egui::Button::new(if self.screen == Screen::Login { "Sign In" } else { "Register" })
                                    .fill(ACCENT),
                            );

                            let enter_pressed = user_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                || pass_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                            if submit.clicked() || enter_pressed {
                                if self.screen == Screen::Login {
                                    self.do_login();
                                } else {
                                    self.do_register();
                                }
                            }

                            if let Some(err) = &self.auth.error {
                                ui.add_space(8.0);
                                ui.colored_label(Color32::from_rgb(240, 71, 71), err);
                            }

                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);

                            if self.screen == Screen::Login {
                                if ui.button("Don't have an account? Register").clicked() {
                                    self.screen = Screen::Register;
                                    self.auth.error = None;
                                }
                            } else if ui.button("Already have an account? Sign In").clicked() {
                                self.screen = Screen::Login;
                                self.auth.error = None;
                            }
                        });
                });
            });
    }

    fn ui_main(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("worlds_panel")
            .resizable(false)
            .exact_width(64.0)
            .frame(egui::Frame::default().fill(BG_DARK))
            .show(ctx, |ui| {
                ui.add_space(8.0);

                if !self.show_friends_sidebar {
                    for world in &self.chat.worlds.clone() {
                        let selected = self.chat.selected_world == Some(world.id);
                        let label = world.name.chars().next().unwrap_or('?').to_uppercase().next().unwrap_or('?');
                        let btn = egui::Button::new(
                            RichText::new(label.to_string()).font(FontId::proportional(20.0)).color(Color32::WHITE)
                        )
                        .fill(if selected { ACCENT } else { BG_MID })
                        .rounding(if selected { 12.0 } else { 24.0 })
                        .min_size(egui::vec2(48.0, 48.0));

                        if ui.add(btn).on_hover_text(&world.name).clicked() && !selected {
                            let wid = world.id;
                            self.chat.selected_world = Some(wid);
                            self.chat.selected_location = None;
                            self.chat.messages.clear();
                            self.main_view = MainView::Welcome;
                            self.load_locations(wid);
                        }
                        ui.add_space(4.0);
                    }

                    ui.add_space(4.0);
                    if ui
                        .add(
                            egui::Button::new(RichText::new("+").font(FontId::proportional(24.0)).color(Color32::GREEN))
                                .fill(BG_MID)
                                .rounding(24.0)
                                .min_size(egui::vec2(48.0, 48.0)),
                        )
                        .on_hover_text("Create World")
                        .clicked()
                    {
                        self.chat.show_create_world = true;
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    let badge = self.friends.unread_requests;
                    let friends_btn = ui.add(
                        egui::Button::new(
                            RichText::new(if badge > 0 { format!("DM\n({})", badge) } else { "DM".into() })
                                .font(FontId::proportional(13.0))
                                .color(if self.show_friends_sidebar { Color32::WHITE } else { TEXT_MUTED })
                        )
                        .fill(if self.show_friends_sidebar { ACCENT } else { BG_MID })
                        .rounding(12.0)
                        .min_size(egui::vec2(48.0, 48.0)),
                    ).on_hover_text("Friends & DMs");

                    if friends_btn.clicked() {
                        self.show_friends_sidebar = !self.show_friends_sidebar;
                        if self.show_friends_sidebar {
                            self.friends.unread_requests = 0;
                        }
                    }
                });
            });

        if self.show_friends_sidebar {
            self.ui_friends_panel(ctx);
        } else {
            self.ui_locations_panel(ctx);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(BG_LIGHT))
            .show(ctx, |ui| {
                match self.main_view {
                    MainView::LocationChat => self.ui_location_chat(ui),
                    MainView::DirectMessage => self.ui_dm_chat(ui),
                    MainView::Welcome => {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("Select a location or friend to start chatting").color(TEXT_MUTED));
                        });
                    }
                }
            });

        self.ui_create_world_modal(ctx);
        self.ui_create_location_modal(ctx);
    }

    fn ui_locations_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("locations_panel")
            .resizable(true)
            .default_width(200.0)
            .frame(egui::Frame::default().fill(BG_MID))
            .show(ctx, |ui| {
                if let Some(wid) = self.chat.selected_world {
                    let world_name = self.chat.worlds.iter()
                        .find(|w| w.id == wid)
                        .map(|w| w.name.as_str())
                        .unwrap_or("World");
                    ui.add_space(8.0);
                    ui.label(RichText::new(world_name).strong().color(Color32::WHITE));
                    ui.separator();
                    ui.label(RichText::new("LOCATIONS").small().color(TEXT_MUTED));
                    ui.add_space(4.0);

                    for loc in &self.chat.locations.clone() {
                        let selected = self.chat.selected_location == Some(loc.id);
                        let resp = ui.selectable_label(selected, RichText::new(format!("# {}", loc.name)).color(
                            if selected { Color32::WHITE } else { TEXT_MUTED },
                        ));
                        if resp.clicked() && !selected {
                            let lid = loc.id;
                            self.chat.selected_location = Some(lid);
                            self.chat.messages.clear();
                            self.main_view = MainView::LocationChat;
                            self.friends.active_dm_peer = None;
                            if let Some(ws) = &self.ws {
                                ws.join(lid);
                            }
                            self.load_messages(wid, lid);
                        }
                    }

                    ui.add_space(8.0);
                    if ui.small_button("+ Add Location").clicked() {
                        self.chat.show_create_location = true;
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Select a world").color(TEXT_MUTED));
                    });
                }
            });
    }

    fn ui_friends_panel(&mut self, ctx: &egui::Context) {
        let mut accept_id: Option<Uuid> = None;
        let mut decline_id: Option<Uuid> = None;
        let mut cancel_id: Option<Uuid> = None;
        let mut open_dm_id: Option<Uuid> = None;
        let mut send_request = false;

        egui::SidePanel::left("friends_panel")
            .resizable(true)
            .default_width(200.0)
            .frame(egui::Frame::default().fill(BG_MID))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new("FRIENDS").small().strong().color(TEXT_MUTED));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    let w = ui.available_width() - 36.0;
                    ui.add(
                        egui::TextEdit::singleline(&mut self.friends.add_friend_input)
                            .desired_width(w)
                            .hint_text("Add by username"),
                    );
                    if ui.add(egui::Button::new("+").fill(ACCENT).min_size(egui::vec2(28.0, 28.0))).clicked()
                        && !self.friends.add_friend_input.trim().is_empty()
                    {
                        send_request = true;
                    }
                });

                if let Some(err) = &self.friends.add_friend_error.clone() {
                    ui.colored_label(Color32::from_rgb(240, 71, 71), err);
                }

                ui.separator();

                if !self.friends.friends.is_empty() {
                    ui.label(RichText::new("ONLINE").small().color(TEXT_MUTED));
                    for friend in &self.friends.friends.clone() {
                        let active = self.friends.active_dm_peer == Some(friend.user_id);
                        let resp = ui.selectable_label(
                            active,
                            RichText::new(&friend.username).color(if active { Color32::WHITE } else { TEXT_MUTED }),
                        );
                        if resp.clicked() {
                            open_dm_id = Some(friend.user_id);
                        }
                    }
                    ui.add_space(4.0);
                }

                if !self.friends.pending_incoming.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(format!("REQUESTS ({})", self.friends.pending_incoming.len())).small().color(TEXT_MUTED));
                    for req in &self.friends.pending_incoming.clone() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&req.requester_name).color(Color32::WHITE));
                            if ui.small_button("Accept").clicked() {
                                accept_id = Some(req.id);
                            }
                            if ui.small_button("Decline").clicked() {
                                decline_id = Some(req.id);
                            }
                        });
                    }
                }

                if !self.friends.pending_outgoing.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("SENT").small().color(TEXT_MUTED));
                    for req in &self.friends.pending_outgoing.clone() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&req.addressee_name).color(TEXT_MUTED));
                            if ui.small_button("Cancel").clicked() {
                                cancel_id = Some(req.id);
                            }
                        });
                    }
                }
            });

        if send_request { self.do_send_friend_request(); }
        if let Some(id) = accept_id { self.do_accept_friend_request(id); }
        if let Some(id) = decline_id { self.do_remove_friend(id); }
        if let Some(id) = cancel_id { self.do_remove_friend(id); }
        if let Some(id) = open_dm_id { self.open_dm(id); }
    }

    fn ui_location_chat(&mut self, ui: &mut egui::Ui) {
        if let Some(loc_id) = self.chat.selected_location {
            let loc_name = self.chat.locations.iter()
                .find(|l| l.id == loc_id)
                .map(|l| l.name.clone())
                .unwrap_or_default();

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("# {}", loc_name)).strong().color(Color32::WHITE));
            });
            ui.separator();

            let available = ui.available_height() - 60.0;
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(available)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for msg in &self.chat.messages {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&msg.sender_name).strong().color(ACCENT));
                            ui.label(RichText::new(msg.timestamp.format("%H:%M").to_string()).small().color(TEXT_MUTED));
                        });
                        ui.label(&msg.content);
                        ui.add_space(4.0);
                    }
                });

            ui.separator();
            let compose_width = ui.available_width() - 80.0;
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.chat.compose_text)
                        .desired_width(compose_width)
                        .hint_text(format!("Message #{}", loc_name)),
                );
                let send_clicked = ui.add(egui::Button::new("Send").fill(ACCENT).min_size(egui::vec2(70.0, 28.0))).clicked();
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (send_clicked || enter) && !self.chat.compose_text.trim().is_empty() {
                    if let Some(ws) = &self.ws {
                        ws.send_chat(loc_id, self.chat.compose_text.trim().to_owned());
                    }
                    self.chat.compose_text.clear();
                    resp.request_focus();
                }
            });
        }
    }

    fn ui_dm_chat(&mut self, ui: &mut egui::Ui) {
        if let Some(peer_id) = self.friends.active_dm_peer {
            let peer_name = self.friends.peer_username(peer_id).to_owned();

            ui.label(RichText::new(format!("@ {}", peer_name)).strong().color(Color32::WHITE));
            ui.separator();

            let available = ui.available_height() - 60.0;
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(available)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for dm in &self.friends.dm_messages {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&dm.sender_name).strong().color(ACCENT));
                            ui.label(RichText::new(dm.timestamp.format("%H:%M").to_string()).small().color(TEXT_MUTED));
                        });
                        ui.label(&dm.content);
                        ui.add_space(4.0);
                    }
                });

            ui.separator();
            let compose_width = ui.available_width() - 80.0;
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.friends.dm_compose)
                        .desired_width(compose_width)
                        .hint_text(format!("Message @{}", peer_name)),
                );
                let send_clicked = ui.add(egui::Button::new("Send").fill(ACCENT).min_size(egui::vec2(70.0, 28.0))).clicked();
                let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (send_clicked || enter) && !self.friends.dm_compose.trim().is_empty() {
                    if let Some(ws) = &self.ws {
                        ws.send_dm(peer_id, self.friends.dm_compose.trim().to_owned());
                    }
                    self.friends.dm_compose.clear();
                    resp.request_focus();
                }
            });
        }
    }

    fn ui_create_world_modal(&mut self, ctx: &egui::Context) {
        if !self.chat.show_create_world {
            return;
        }
        egui::Window::new("Create World")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut self.chat.new_world_name);
                ui.label("Description");
                ui.text_edit_multiline(&mut self.chat.new_world_description);
                ui.label("Theme");
                egui::ComboBox::from_id_salt("theme_combo")
                    .selected_text(&self.chat.new_world_theme)
                    .show_ui(ui, |ui| {
                        for theme in ["fantasy", "cyberpunk", "scifi", "horror", "superhero", "post-apocalyptic", "noir", "custom"] {
                            ui.selectable_value(&mut self.chat.new_world_theme, theme.to_owned(), theme);
                        }
                    });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.chat.show_create_world = false;
                    }
                    if ui.add(egui::Button::new("Create").fill(ACCENT)).clicked()
                        && !self.chat.new_world_name.trim().is_empty()
                    {
                        let token = self.auth.token.clone();
                        let name = self.chat.new_world_name.trim().to_owned();
                        let desc = self.chat.new_world_description.trim().to_owned();
                        let theme = self.chat.new_world_theme.clone();
                        let pending = Arc::clone(&self.pending);
                        self.rt.spawn(async move {
                            let result = match api::create_world(&token, &name, &desc, &theme).await {
                                Ok(w) => PendingResult::WorldCreated(w),
                                Err(e) => PendingResult::ApiError(e.to_string()),
                            };
                            pending.lock().unwrap().push(result);
                        });
                        self.chat.new_world_name.clear();
                        self.chat.new_world_description.clear();
                    }
                });
            });
    }

    fn ui_create_location_modal(&mut self, ctx: &egui::Context) {
        if !self.chat.show_create_location {
            return;
        }
        egui::Window::new("Create Location")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Location Name");
                ui.text_edit_singleline(&mut self.chat.new_location_name);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.chat.show_create_location = false;
                    }
                    if ui.add(egui::Button::new("Create").fill(ACCENT)).clicked()
                        && !self.chat.new_location_name.trim().is_empty()
                    {
                        if let Some(wid) = self.chat.selected_world {
                            let token = self.auth.token.clone();
                            let name = self.chat.new_location_name.trim().to_owned();
                            let pending = Arc::clone(&self.pending);
                            self.rt.spawn(async move {
                                let result = match api::create_location(&token, wid, &name).await {
                                    Ok(l) => PendingResult::LocationCreated(l),
                                    Err(e) => PendingResult::ApiError(e.to_string()),
                                };
                                pending.lock().unwrap().push(result);
                            });
                            self.chat.new_location_name.clear();
                        }
                    }
                });
            });
    }
}

impl eframe::App for RealmChatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_pending(ctx);

        ctx.set_visuals({
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = BG_DARK;
            visuals.window_fill = BG_MID;
            visuals.override_text_color = Some(Color32::from_rgb(220, 221, 222));
            visuals
        });

        match self.screen {
            Screen::Login | Screen::Register => self.ui_login(ctx),
            Screen::Main => self.ui_main(ctx),
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
