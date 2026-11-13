//! EDOPro's core (`ocgapi.h`), linked in by `build.rs`, behind a safe [`Duel`]. The card and script readers and the log
//! handler call back into a [`Host`] the duel owns.
use std::ffi::{CStr, CString, c_char, c_int, c_void};

#[repr(C)]
struct CardData {
    code: u32,
    alias: u32,
    setcodes: *mut u16,
    kind: u32,
    level: u32,
    attribute: u32,
    race: u64,
    attack: i32,
    defense: i32,
    lscale: u32,
    rscale: u32,
    link_marker: u32,
}

#[repr(C)]
struct Player {
    starting_lp: u32,
    starting_draw_count: u32,
    draw_count_per_turn: u32,
}

type DataReader = unsafe extern "C" fn(*mut c_void, u32, *mut CardData);
type DataReaderDone = unsafe extern "C" fn(*mut c_void, *mut CardData);
type ScriptReader = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> c_int;
type LogHandler = unsafe extern "C" fn(*mut c_void, *const c_char, c_int);

#[repr(C)]
struct DuelOptions {
    seed: [u64; 4],
    flags: u64,
    team1: Player,
    team2: Player,
    card_reader: DataReader,
    payload1: *mut c_void,
    script_reader: ScriptReader,
    payload2: *mut c_void,
    log_handler: LogHandler,
    payload3: *mut c_void,
    card_reader_done: DataReaderDone,
    payload4: *mut c_void,
    enable_unsafe_libraries: u8,
}

#[repr(C)]
struct NewCardInfo {
    team: u8,
    duelist: u8,
    code: u32,
    con: u8,
    loc: u32,
    seq: u32,
    pos: u32,
}

#[repr(C)]
struct QueryInfo {
    flags: u32,
    con: u8,
    loc: u32,
    seq: u32,
    overlay_seq: u32,
}

#[cfg(not(no_core))]
unsafe extern "C" {
    fn OCG_CreateDuel(out: *mut *mut c_void, options: *const DuelOptions) -> c_int;
    fn OCG_DestroyDuel(duel: *mut c_void);
    fn OCG_DuelNewCard(duel: *mut c_void, info: *const NewCardInfo);
    fn OCG_StartDuel(duel: *mut c_void);
    fn OCG_DuelProcess(duel: *mut c_void) -> c_int;
    fn OCG_DuelGetMessage(duel: *mut c_void, length: *mut u32) -> *mut c_void;
    fn OCG_DuelSetResponse(duel: *mut c_void, buffer: *const c_void, length: u32);
    fn OCG_LoadScript(duel: *mut c_void, buffer: *const c_char, length: u32, name: *const c_char) -> c_int;
    fn OCG_DuelQueryCount(duel: *mut c_void, team: u8, loc: u32) -> u32;
    fn OCG_DuelQuery(duel: *mut c_void, length: *mut u32, info: *const QueryInfo) -> *mut c_void;
    fn OCG_DuelQueryLocation(duel: *mut c_void, length: *mut u32, info: *const QueryInfo) -> *mut c_void;
    fn OCG_DuelQueryField(duel: *mut c_void, length: *mut u32) -> *mut c_void;
}

/// A build without the core (`no_core`, see build.rs): the program says so instead of linking.
#[cfg(no_core)]
#[expect(non_snake_case, clippy::missing_safety_doc, reason = "stand-ins with the core's own names")]
mod missing {
    use super::*;
    pub unsafe fn OCG_CreateDuel(_: *mut *mut c_void, _: *const DuelOptions) -> c_int { 2 }
    pub unsafe fn OCG_DestroyDuel(_: *mut c_void) {}
    pub unsafe fn OCG_DuelNewCard(_: *mut c_void, _: *const NewCardInfo) {}
    pub unsafe fn OCG_StartDuel(_: *mut c_void) {}
    pub unsafe fn OCG_DuelProcess(_: *mut c_void) -> c_int { 0 }
    pub unsafe fn OCG_DuelGetMessage(_: *mut c_void, length: *mut u32) -> *mut c_void { unsafe { *length = 0 }; std::ptr::null_mut() }
    pub unsafe fn OCG_DuelSetResponse(_: *mut c_void, _: *const c_void, _: u32) {}
    pub unsafe fn OCG_LoadScript(_: *mut c_void, _: *const c_char, _: u32, _: *const c_char) -> c_int { 0 }
    pub unsafe fn OCG_DuelQueryCount(_: *mut c_void, _: u8, _: u32) -> u32 { 0 }
    pub unsafe fn OCG_DuelQuery(_: *mut c_void, length: *mut u32, _: *const QueryInfo) -> *mut c_void { unsafe { *length = 0 }; std::ptr::null_mut() }
    pub unsafe fn OCG_DuelQueryLocation(_: *mut c_void, length: *mut u32, _: *const QueryInfo) -> *mut c_void { unsafe { *length = 0 }; std::ptr::null_mut() }
    pub unsafe fn OCG_DuelQueryField(_: *mut c_void, length: *mut u32) -> *mut c_void { unsafe { *length = 0 }; std::ptr::null_mut() }
}
#[cfg(no_core)]
use missing::*;

/// Whether this program was built with the core.
pub const BUILT_WITH_CORE: bool = cfg!(not(no_core));

/// A card's data for the core.
#[derive(Default, Clone)]
pub struct Card {
    pub code: u32,
    pub alias: u32,
    /// As the core reads them: up to the first 0.
    pub setcodes: Vec<u16>,
    pub kind: u32,
    pub level: u32,
    pub attribute: u32,
    pub race: u64,
    pub attack: i32,
    pub defense: i32,
    pub lscale: u32,
    pub rscale: u32,
}

/// What a duel asks of its owner while it runs.
pub trait Host {
    /// Card data, or None for a code the database lacks (the core then gets zeros).
    fn card(&mut self, code: u32) -> Option<Card>;
    /// A script's text by file name (`c<passcode>.lua`, `constant.lua`...), or None.
    fn script(&mut self, name: &str) -> Option<Vec<u8>>;
    /// The core's log: its type (0 error, 1 from a script, 2 for debugging) and text.
    fn log(&mut self, kind: i32, text: &str);
}

struct Callbacks {
    host: Box<dyn Host>,
}

unsafe extern "C" fn read_card(payload: *mut c_void, code: u32, data: *mut CardData) {
    // SAFETY: payload is the duel's boxed Callbacks, alive as long as the duel; data is the core's to fill.
    let callbacks = unsafe { &mut *(payload as *mut Callbacks) };
    let card = callbacks.host.card(code).unwrap_or_default();
    let setcodes = if card.setcodes.first().is_none_or(|s| *s == 0) {
        std::ptr::null_mut()
    } else {
        // The core reads up to the first 0, so the list stops there; read_card_done finds its length the same way.
        let mut list: Vec<u16> = card.setcodes.iter().copied().take_while(|s| *s != 0).collect();
        list.push(0);
        Box::into_raw(list.into_boxed_slice()) as *mut u16
    };
    let out = CardData {
        code: card.code,
        alias: card.alias,
        setcodes,
        kind: card.kind,
        level: card.level,
        attribute: card.attribute,
        race: card.race,
        attack: card.attack,
        defense: card.defense,
        lscale: card.lscale,
        rscale: card.rscale,
        link_marker: 0,
    };
    unsafe { data.write(out) };
}

unsafe extern "C" fn read_card_done(_: *mut c_void, data: *mut CardData) {
    // SAFETY: the setcodes read_card allocated: a boxed slice of nonzero codes and its terminating 0.
    let setcodes = unsafe { (*data).setcodes };
    if setcodes.is_null() {
        return;
    }
    let mut len = 0;
    while unsafe { *setcodes.add(len) } != 0 {
        len += 1;
    }
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(setcodes, len + 1)) });
    unsafe { (*data).setcodes = std::ptr::null_mut() };
}

unsafe extern "C" fn read_script(payload: *mut c_void, duel: *mut c_void, name: *const c_char) -> c_int {
    // SAFETY: as read_card; name is the core's NUL-terminated file name.
    let callbacks = unsafe { &mut *(payload as *mut Callbacks) };
    let name = unsafe { CStr::from_ptr(name) };
    let Some(text) = callbacks.host.script(&name.to_string_lossy()) else { return 0 };
    unsafe { OCG_LoadScript(duel, text.as_ptr() as *const c_char, text.len() as u32, name.as_ptr()) }
}

unsafe extern "C" fn log_handler(payload: *mut c_void, text: *const c_char, kind: c_int) {
    // SAFETY: as read_card; text is the core's NUL-terminated message.
    let callbacks = unsafe { &mut *(payload as *mut Callbacks) };
    let text = unsafe { CStr::from_ptr(text) };
    callbacks.host.log(kind, &text.to_string_lossy());
}

/// One duel of the core.
pub struct Duel {
    handle: *mut c_void,
    callbacks: *mut Callbacks,
}

/// What [`Duel::process`] returns.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    End,
    Awaiting,
    Continue,
}

impl Duel {
    /// A duel with `flags` (the duel mode), both teams' LP and draws; None when the core refuses it.
    pub fn new(seed: [u64; 4], flags: u64, lp: u32, start_draw: u32, host: Box<dyn Host>) -> Option<Duel> {
        let callbacks = Box::into_raw(Box::new(Callbacks { host }));
        let team = || Player { starting_lp: lp, starting_draw_count: start_draw, draw_count_per_turn: 1 };
        let options = DuelOptions {
            seed,
            flags,
            team1: team(),
            team2: team(),
            card_reader: read_card,
            payload1: callbacks as *mut c_void,
            script_reader: read_script,
            payload2: callbacks as *mut c_void,
            log_handler,
            payload3: callbacks as *mut c_void,
            card_reader_done: read_card_done,
            payload4: callbacks as *mut c_void,
            // As ocgcore-wasm creates its duels (the io library besides the safe ones).
            enable_unsafe_libraries: 1,
        };
        let mut handle = std::ptr::null_mut();
        // SAFETY: options is fully initialised and the callbacks outlive the duel (freed in Drop).
        let status = unsafe { OCG_CreateDuel(&mut handle, &options) };
        if status != 0 || handle.is_null() {
            drop(unsafe { Box::from_raw(callbacks) });
            return None;
        }
        Some(Duel { handle, callbacks })
    }

    pub fn load_script(&mut self, name: &str, text: &[u8]) -> bool {
        let name = CString::new(name).unwrap_or_default();
        unsafe { OCG_LoadScript(self.handle, text.as_ptr() as *const c_char, text.len() as u32, name.as_ptr()) == 1 }
    }

    #[expect(clippy::too_many_arguments, reason = "the core's OCG_NewCardInfo, field by field")]
    pub fn new_card(&mut self, team: u8, duelist: u8, code: u32, controller: u8, location: u32, sequence: u32, position: u32) {
        let info = NewCardInfo { team, duelist, code, con: controller, loc: location, seq: sequence, pos: position };
        unsafe { OCG_DuelNewCard(self.handle, &info) }
    }

    pub fn start(&mut self) {
        unsafe { OCG_StartDuel(self.handle) }
    }

    pub fn process(&mut self) -> Status {
        match unsafe { OCG_DuelProcess(self.handle) } {
            0 => Status::End,
            1 => Status::Awaiting,
            _ => Status::Continue,
        }
    }

    /// The messages since the last call: each a u32 length and its bytes.
    pub fn messages(&mut self) -> Vec<u8> {
        let mut length = 0;
        let buffer = unsafe { OCG_DuelGetMessage(self.handle, &mut length) };
        copy(buffer, length)
    }

    pub fn respond(&mut self, response: &[u8]) {
        unsafe { OCG_DuelSetResponse(self.handle, response.as_ptr() as *const c_void, response.len() as u32) }
    }

    pub fn count(&mut self, team: u8, location: u32) -> u32 {
        unsafe { OCG_DuelQueryCount(self.handle, team, location) }
    }

    /// One card's query (empty: a u16 0).
    pub fn query(&mut self, flags: u32, controller: u8, location: u32, sequence: u32, overlay_sequence: u32) -> Vec<u8> {
        let info = QueryInfo { flags, con: controller, loc: location, seq: sequence, overlay_seq: overlay_sequence };
        let mut length = 0;
        let buffer = unsafe { OCG_DuelQuery(self.handle, &mut length, &info) };
        copy(buffer, length)
    }

    /// A location's queries: a u32 size, then one query per card or slot.
    pub fn query_location(&mut self, flags: u32, controller: u8, location: u32) -> Vec<u8> {
        let info = QueryInfo { flags, con: controller, loc: location, seq: 0, overlay_seq: 0 };
        let mut length = 0;
        let buffer = unsafe { OCG_DuelQueryLocation(self.handle, &mut length, &info) };
        copy(buffer, length)
    }

    pub fn query_field(&mut self) -> Vec<u8> {
        let mut length = 0;
        let buffer = unsafe { OCG_DuelQueryField(self.handle, &mut length) };
        copy(buffer, length)
    }
}

fn copy(buffer: *mut c_void, length: u32) -> Vec<u8> {
    if buffer.is_null() || length == 0 {
        return vec![];
    }
    // SAFETY: the core's buffer of `length` bytes, valid until its next call.
    unsafe { std::slice::from_raw_parts(buffer as *const u8, length as usize) }.to_vec()
}

impl Drop for Duel {
    fn drop(&mut self) {
        unsafe {
            OCG_DestroyDuel(self.handle);
            drop(Box::from_raw(self.callbacks));
        }
    }
}
