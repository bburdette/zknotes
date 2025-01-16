use crate::{
  content::{
    ArchiveZkLink, GetArchiveZkLinks, GetArchiveZkNote, GetZkLinksSince, GetZkNoteAndLinks,
    GetZkNoteArchives, GetZkNoteComments, GetZnlIfChanged, ImportZkNote, JobStatus, SaveZkNote,
    SaveZkNoteAndLinks, SavedZkNote, ZkLinks, ZkListNote, ZkNote, ZkNoteAndLinksWhat,
    ZkNoteArchives, ZkNoteId,
  },
  search::{
    TagSearch, ZkIdSearchResult, ZkListNoteSearchResult, ZkNoteAndLinksSearchResult, ZkNoteSearch,
    ZkNoteSearchResult,
  },
};
use elm_rs::{Elm, ElmDecode, ElmEncode};
use serde_derive::{Deserialize, Serialize};
// use uuid::Uuid;

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Debug)]
pub enum PrivateRequest {
  PvqGetZkNote(ZkNoteId),
  PvqGetZkNoteAndLinks(GetZkNoteAndLinks),
  PvqGetZnlIfChanged(GetZnlIfChanged),
  PvqGetZkNoteComments(GetZkNoteComments),
  PvqGetZkNoteArchives(GetZkNoteArchives),
  PvqGetArchiveZkNote(GetArchiveZkNote),
  PvqGetArchiveZklinks(GetArchiveZkLinks),
  PvqGetZkLinksSince(GetZkLinksSince),
  PvqSearchZkNotes(ZkNoteSearch),
  PvqPowerDelete(TagSearch),
  PvqDeleteZkNote(ZkNoteId),
  PvqSaveZkNote(SaveZkNote),
  PvqSaveZkLinks(ZkLinks),
  PvqSaveZkNoteAndLinks(SaveZkNoteAndLinks),
  PvqSaveImportZkNotes(Vec<ImportZkNote>),
  PvqSetHomeNote(ZkNoteId),
  PvqSyncRemote,
  PvqSyncFiles(ZkNoteSearch),
  PvqGetJobStatus(i64),
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Debug)]
pub enum PrivateReply {
  PvyServerError(PrivateError),
  PvyZkNote(ZkNote),
  PvyZkNoteAndLinksWhat(ZkNoteAndLinksWhat),
  PvyNoop,
  PvyZkNoteComments(Vec<ZkNote>),
  PvyArchives(Vec<ZkListNote>),
  PvyZkNoteArchives(ZkNoteArchives),
  PvyArchiveZkLinks(Vec<ArchiveZkLink>),
  PvyZkLinks(ZkLinks),
  PvyZkListNoteSearchResult(ZkListNoteSearchResult),
  PvyZkNoteSearchResult(ZkNoteSearchResult),
  PvyZkNoteIdSearchResult(ZkIdSearchResult),
  PvyZkNoteAndLinksSearchResult(ZkNoteAndLinksSearchResult),
  PvyPowerDeleteComplete(i64),
  PvyDeletedZkNote(ZkNoteId),
  PvySavedZkNote(SavedZkNote),
  PvySavedZkLinks,
  PvySavedZkNoteAndLinks,
  PvySavedImportZkNotes,
  PvyHomeNoteSet(ZkNoteId),
  PvyJobStatus(JobStatus),
  PvyJobNotFound(i64),
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct ZkNoteRq {
  zknoteid: ZkNoteId,
  what: Option<String>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PrivateError {
  String(String),
  NoteNotFound(ZkNoteRq),
  NoteIsPrivate(ZkNoteRq),
}
