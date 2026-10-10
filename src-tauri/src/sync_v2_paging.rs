//! Bounded, restart-safe encrypted page assembly. No live cursor advances here.
#![allow(dead_code)]
use crate::{
    error::AppResult,
    persistence::Database,
    sync_v2_local_queue::{decode, encode, scope, PulledRecord, StreamScope},
    sync_v2_record_codec::{invalid, open_record, MAX_SAFE_INTEGER},
    sync_v2_record_protocol::SealedWorkspaceCheckpoint,
};
use rusqlite::{params, OptionalExtension};
impl Database {
    pub(crate) fn buffer_sync_v2_page(
        &mut self,
        s: StreamScope,
        start_after: u64,
        page: &[PulledRecord],
        checkpoint: &SealedWorkspaceCheckpoint,
        key: &[u8; 32],
    ) -> AppResult<bool> {
        if page.is_empty()
            || page.len() > 100
            || start_after > MAX_SAFE_INTEGER
            || checkpoint.protocol_version != 1
            || checkpoint.checkpoint_counter == 0
            || checkpoint.checkpoint_counter > MAX_SAFE_INTEGER
            || checkpoint.through_change_seq <= start_after
            || checkpoint.through_change_seq > MAX_SAFE_INTEGER
            || checkpoint.through_change_seq - start_after > 10_000
            || checkpoint.ciphertext.len() > 4096
        {
            return Err(invalid());
        }
        let checkpoint_bytes = serde_json::to_vec(checkpoint).map_err(|_| invalid())?;
        for (i, c) in page.iter().enumerate() {
            crate::sync_v2_record_codec::validate_header(&c.record.header)?;
            if c.record.header.owner_id != s.owner_id
                || c.record.header.workspace_id != s.workspace_id
                || c.change_seq <= start_after
                || c.change_seq > checkpoint.through_change_seq
                || c.server_version != c.record.header.expected_server_version + 1
                || (i > 0 && c.change_seq != page[i - 1].change_seq + 1)
            {
                return Err(invalid());
            }
            open_record(&c.record, key)?;
        }
        let tx = self.connection.transaction().map_err(|_| invalid())?;
        scope(&tx, s)?;
        let cursor: u64 = tx
            .query_row(
                "SELECT pull_cursor FROM sync_v2_local_streams WHERE workspace_id=?1",
                [s.workspace_id.to_string()],
                |r| Ok(r.get::<_, i64>(0)? as u64),
            )
            .map_err(|_| invalid())?;
        if cursor != start_after && cursor != checkpoint.through_change_seq {
            return Err(invalid());
        }
        let batch: Option<(u64, Vec<u8>)> = tx
            .query_row(
                "SELECT start_after,checkpoint FROM sync_v2_pull_batches WHERE workspace_id=?1",
                [s.workspace_id.to_string()],
                |r| Ok((r.get::<_, i64>(0)? as u64, r.get(1)?)),
            )
            .optional()
            .map_err(|_| invalid())?;
        if let Some(batch) = batch {
            if batch != (start_after, checkpoint_bytes.clone()) {
                return Err(invalid());
            }
        } else {
            tx.execute(
                "INSERT INTO sync_v2_pull_batches VALUES(?1,?2,?3)",
                params![
                    s.workspace_id.to_string(),
                    start_after as i64,
                    checkpoint_bytes
                ],
            )
            .map_err(|_| invalid())?;
        }
        let last: u64 = tx
            .query_row(
                "SELECT COALESCE(MAX(change_seq),?2) FROM sync_v2_pull_parts WHERE workspace_id=?1",
                params![s.workspace_id.to_string(), start_after as i64],
                |r| Ok(r.get::<_, i64>(0)? as u64),
            )
            .map_err(|_| invalid())?;
        let mut next = last + 1;
        for c in page {
            let bytes = encode(&c.record)?;
            let old:Option<(u64,Vec<u8>)>=tx.query_row("SELECT server_version,envelope FROM sync_v2_pull_parts WHERE workspace_id=?1 AND change_seq=?2",params![s.workspace_id.to_string(),c.change_seq as i64],|r|Ok((r.get::<_,i64>(0)? as u64,r.get(1)?))).optional().map_err(|_|invalid())?;
            if let Some(old) = old {
                if old != (c.server_version, bytes) {
                    return Err(invalid());
                }
            } else {
                if c.change_seq != next {
                    return Err(invalid());
                }
                next += 1;
                tx.execute(
                    "INSERT INTO sync_v2_pull_parts VALUES(?1,?2,?3,?4)",
                    params![
                        s.workspace_id.to_string(),
                        c.change_seq as i64,
                        c.server_version as i64,
                        bytes
                    ],
                )
                .map_err(|_| invalid())?;
            }
        }
        let (count,size):(u64,u64)=tx.query_row("SELECT COUNT(*),COALESCE(SUM(length(envelope)),0) FROM sync_v2_pull_parts WHERE workspace_id=?1",[s.workspace_id.to_string()],|r|Ok((r.get::<_,i64>(0)? as u64,r.get::<_,i64>(1)? as u64))).map_err(|_|invalid())?;
        if count > 10_000 || size > 128 * 1024 * 1024 {
            return Err(invalid());
        }
        tx.commit().map_err(|_| invalid())?;
        Ok(count == checkpoint.through_change_seq - start_after)
    }
    pub(crate) fn buffered_sync_v2_batch(
        &self,
        s: StreamScope,
    ) -> AppResult<(u64, Vec<PulledRecord>, SealedWorkspaceCheckpoint)> {
        let (after,bytes):(u64,Vec<u8>)=self.connection.query_row("SELECT b.start_after,b.checkpoint FROM sync_v2_pull_batches b JOIN sync_v2_local_streams s USING(workspace_id) WHERE b.workspace_id=?1 AND s.owner_id=?2 AND s.device_id=?3",params![s.workspace_id.to_string(),s.owner_id.to_string(),s.device_id.to_string()],|r|Ok((r.get::<_,i64>(0)? as u64,r.get(1)?))).map_err(|_|invalid())?;
        let cp: SealedWorkspaceCheckpoint =
            serde_json::from_slice(&bytes).map_err(|_| invalid())?;
        let mut stmt=self.connection.prepare("SELECT change_seq,server_version,envelope FROM sync_v2_pull_parts WHERE workspace_id=?1 ORDER BY change_seq").map_err(|_|invalid())?;
        let rows = stmt
            .query_map([s.workspace_id.to_string()], |r| {
                Ok((
                    r.get::<_, i64>(0)? as u64,
                    r.get::<_, i64>(1)? as u64,
                    r.get::<_, Vec<u8>>(2)?,
                ))
            })
            .map_err(|_| invalid())?;
        let records = rows
            .map(|row| {
                let (change_seq, server_version, bytes) = row.map_err(|_| invalid())?;
                Ok(PulledRecord {
                    change_seq,
                    server_version,
                    record: decode(&bytes)?,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok((after, records, cp))
    }
}
