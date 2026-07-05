use libc::c_char;
use once_cell::sync::Lazy;
use std::ffi::{CStr, CString};
use tokio::runtime::Runtime;

use lavende::{LavendeManager, LavendePlayer};

static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().unwrap());

pub type SendToShardCb = extern "C" fn(*const c_char, *const c_char);
pub type EventCb = extern "C" fn(*const c_char);

fn cstr_to_string(c_str: *const c_char) -> String {
    if c_str.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(c_str).to_string_lossy().into_owned() }
}

fn string_to_cstr(s: String) -> *mut c_char {
    CString::new(s).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn lavende_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[no_mangle]
pub extern "C" fn lavende_manager_new(
    client_id: *const c_char,
    send_cb: SendToShardCb,
) -> *mut LavendeManager {
    let client_id_str = cstr_to_string(client_id);
    let manager = LavendeManager::new(client_id_str, move |guild_id, payload| {
        let gid_c = CString::new(guild_id).unwrap();
        let payload_str = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
        let payload_c = CString::new(payload_str).unwrap();
        send_cb(gid_c.as_ptr(), payload_c.as_ptr());
    });
    Box::into_raw(Box::new(manager))
}

#[no_mangle]
pub extern "C" fn lavende_manager_free(ptr: *mut LavendeManager) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn lavende_manager_listen_events(ptr: *mut LavendeManager, event_cb: EventCb) {
    if ptr.is_null() {
        return;
    }
    let manager = unsafe { &*ptr };
    let mut rx = manager.subscribe_events();
    
    RUNTIME.spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
            if let Ok(c_str) = CString::new(json) {
                event_cb(c_str.as_ptr());
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn lavende_manager_get_or_create_player(
    ptr: *mut LavendeManager,
    guild_id: *const c_char,
) -> *mut LavendePlayer {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let manager = unsafe { &*ptr };
    let guild_id_str = cstr_to_string(guild_id);
    let player = manager.get_or_create_player(&guild_id_str);
    Box::into_raw(Box::new(player))
}

#[no_mangle]
pub extern "C" fn lavende_manager_destroy_player(
    ptr: *mut LavendeManager,
    guild_id: *const c_char,
) {
    if ptr.is_null() {
        return;
    }
    let manager = unsafe { &*ptr };
    let guild_id_str = cstr_to_string(guild_id);
    RUNTIME.block_on(async {
        manager.destroy_player(&guild_id_str).await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_manager_send_raw_data(
    ptr: *mut LavendeManager,
    packet_json: *const c_char,
) {
    if ptr.is_null() {
        return;
    }
    let manager = unsafe { &*ptr };
    let json_str = cstr_to_string(packet_json);
    if let Ok(val) = serde_json::from_str(&json_str) {
        RUNTIME.block_on(async {
            manager.send_raw_data(&val).await;
        });
    }
}

#[no_mangle]
pub extern "C" fn lavende_player_free(ptr: *mut LavendePlayer) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn lavende_player_connect(
    ptr: *mut LavendePlayer,
    channel_id: *const c_char,
    self_deaf: bool,
    self_mute: bool,
) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    let channel_id_str = if channel_id.is_null() {
        None
    } else {
        let s = cstr_to_string(channel_id);
        if s.is_empty() { None } else { Some(s) }
    };
    RUNTIME.block_on(async {
        player.connect(channel_id_str, self_deaf, self_mute).await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_disconnect(ptr: *mut LavendePlayer) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    RUNTIME.block_on(async {
        player.disconnect().await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_destroy(ptr: *mut LavendePlayer, reason: *const c_char) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    let reason_str = if reason.is_null() {
        None
    } else {
        Some(cstr_to_string(reason))
    };
    RUNTIME.block_on(async {
        player.destroy(reason_str).await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_play(ptr: *mut LavendePlayer) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let player = unsafe { &*ptr };
    let result = RUNTIME.block_on(async { player.play().await });
    match result {
        Ok(_) => std::ptr::null_mut(),
        Err(e) => string_to_cstr(e),
    }
}

#[no_mangle]
pub extern "C" fn lavende_player_play_track(
    ptr: *mut LavendePlayer,
    track_json: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let player = unsafe { &*ptr };
    let track_json_str = cstr_to_string(track_json);
    let track: Result<lavende::Track, _> = serde_json::from_str(&track_json_str);
    if let Err(e) = track {
        return string_to_cstr(format!("Failed to parse track: {}", e));
    }
    let track = track.unwrap();
    
    RUNTIME.block_on(async {
        let mut q = player.queue.write().await;
        q.current = Some(track);
    });

    let result = RUNTIME.block_on(async { player.play().await });
    match result {
        Ok(_) => std::ptr::null_mut(),
        Err(e) => string_to_cstr(e),
    }
}

#[no_mangle]
pub extern "C" fn lavende_player_pause(ptr: *mut LavendePlayer, state: bool) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    RUNTIME.block_on(async {
        player.pause(state).await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_resume(ptr: *mut LavendePlayer) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    RUNTIME.block_on(async {
        player.resume().await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_stop(ptr: *mut LavendePlayer) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    RUNTIME.block_on(async {
        player.stop().await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_skip(ptr: *mut LavendePlayer) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    RUNTIME.block_on(async {
        player.skip().await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_seek(ptr: *mut LavendePlayer, position_ms: i64) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    RUNTIME.block_on(async {
        player.seek(position_ms).await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_set_volume(ptr: *mut LavendePlayer, volume: u32) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    RUNTIME.block_on(async {
        player.set_volume(volume).await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_set_filters(ptr: *mut LavendePlayer, filters_json: *const c_char) {
    if ptr.is_null() {
        return;
    }
    let player = unsafe { &*ptr };
    let json_str = cstr_to_string(filters_json);
    RUNTIME.block_on(async {
        player.set_filters(json_str).await;
    });
}

#[no_mangle]
pub extern "C" fn lavende_player_get_position(ptr: *mut LavendePlayer) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    let player = unsafe { &*ptr };
    player.get_position()
}

#[no_mangle]
pub extern "C" fn lavende_player_is_paused(ptr: *mut LavendePlayer) -> bool {
    if ptr.is_null() {
        return false;
    }
    let player = unsafe { &*ptr };
    player.is_paused()
}

#[no_mangle]
pub extern "C" fn lavende_player_search(
    ptr: *mut LavendePlayer,
    query: *const c_char,
) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let player = unsafe { &*ptr };
    let query_str = cstr_to_string(query);
    let result = RUNTIME.block_on(async { player.search(&query_str).await });
    let json = match result {
        Ok(res) => serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string()),
        Err(e) => format!(r#"{{"error":"{}"}}"#, e),
    };
    string_to_cstr(json)
}
