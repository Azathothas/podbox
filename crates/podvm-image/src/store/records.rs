use super::*;

impl Store {
    // ----------------------------------------------------------- the queries

    /// Every record, newest pull first.
    pub fn list(&self) -> Result<Vec<Record>> {
        let mut images = self.read_index()?.images;
        images.sort_by(|a, b| {
            b.pulled_at
                .cmp(&a.pulled_at)
                .then(a.repository.cmp(&b.repository))
                .then(a.tag.cmp(&b.tag))
        });
        Ok(images)
    }

    /// Records a reference names. A tag or digest names at most one; a bare
    /// repository or an image id may name several.
    ///
    /// ⚠ Selected by NAME, never by position: `docs/conventions/code.md`.
    pub fn find(&self, want: &str) -> Result<Vec<Record>> {
        let all = self.list()?;
        // An image id, full or docker's twelve-digit short form.
        let id_like = want.len() >= 12
            && want
                .trim_start_matches("sha256:")
                .bytes()
                .all(|b| b.is_ascii_hexdigit());
        if id_like {
            let hex = want.trim_start_matches("sha256:");
            let hits: Vec<Record> = all
                .iter()
                .filter(|r| {
                    r.config_digest
                        .trim_start_matches("sha256:")
                        .starts_with(hex)
                })
                .cloned()
                .collect();
            if !hits.is_empty() {
                return Ok(hits);
            }
        }
        let Ok(reference) = Reference::parse(want) else {
            return Ok(Vec::new());
        };
        let repo = reference.canonical_repository();
        let hits: Vec<Record> = all
            .into_iter()
            .filter(|r| {
                r.repository == repo
                    && match (&reference.digest, &reference.tag) {
                        (Some(d), _) => r.digest == d.to_string(),
                        (None, Some(t)) => r.tag.as_deref() == Some(t.as_str()),
                        (None, None) => true,
                    }
            })
            .collect();
        Ok(hits)
    }

    /// The records a reference names, narrowed to one platform.
    ///
    /// ⛔ `None` means **this machine's**, not "any". A caller that asked for
    /// no platform is asking about the image it could run, and handing it an
    /// arm64 record on an amd64 host is the `Exec format error` this whole
    /// module exists to turn into a sentence. ⚠ Where nothing matches the host
    /// the hits are returned unfiltered rather than emptied, so the caller can
    /// say "podvm holds this image, for another platform" instead of "no such
    /// image", which is a different and more useful failure.
    pub fn find_for(
        &self,
        want: &str,
        platform: Option<&crate::platform::Platform>,
    ) -> Result<Found> {
        let hits = self.find(want)?;
        let host = crate::platform::Platform::host();
        let want_p = platform.unwrap_or(&host);
        // ⛔ Filtered whatever the count. An earlier version short-circuited on
        // a single hit, and a store holding only `linux/amd64` then answered a
        // `--platform linux/arm64` request with the amd64 record: podvm ran the
        // wrong architecture and said nothing. A count is not a match.
        let matched: Vec<Record> = hits
            .iter()
            .filter(|r| r.platform == want_p.to_string())
            .cloned()
            .collect();
        Ok(Found {
            other_platforms: if matched.is_empty() {
                let mut p: Vec<String> = hits.iter().map(|r| r.platform.clone()).collect();
                p.sort_unstable();
                p.dedup();
                p
            } else {
                Vec::new()
            },
            matched,
        })
    }

    pub fn find_one(&self, want: &str) -> Result<Record> {
        self.find_one_for(want, None)
    }

    /// One record, or a refusal that says why there is more than one.
    ///
    /// ⛔ **NEVER `.next()` ON AN AMBIGUOUS REFERENCE.** Until 2026-09-09 this
    /// took the first hit, and that was harmless only because a store could not
    /// hold two records for one tag. [T-0212](../../../TODO/image.md) made it
    /// hold one per platform, and the same line then meant `podvm extract
    /// alpine` silently unpacked whichever platform was pulled most recently:
    /// selected **by position**, which `docs/conventions/code.md` forbids and
    /// which `Store::find`'s own comment calls out three functions above.
    ///
    /// ⚠ The host's platform is preferred rather than demanded, because a store
    /// holding exactly one foreign platform and asked for no platform in
    /// particular is not ambiguous: there is one answer and refusing it would be
    /// pedantry. Ambiguity is two or more surviving that preference.
    pub fn find_one_for(
        &self,
        want: &str,
        platform: Option<&crate::platform::Platform>,
    ) -> Result<Record> {
        let hits = self.find(want)?;
        if hits.is_empty() {
            return Err(Error::NoSuchImage(want.to_string()));
        }
        if hits.len() == 1 && platform.is_none() {
            return Ok(hits.into_iter().next().expect("length checked"));
        }
        let host = crate::platform::Platform::host();
        let prefer = platform.unwrap_or(&host);
        let narrowed: Vec<Record> = hits
            .iter()
            .filter(|r| r.platform == prefer.to_string())
            .cloned()
            .collect();
        match narrowed.len() {
            1 => Ok(narrowed.into_iter().next().expect("length checked")),
            0 if platform.is_some() => {
                let mut have: Vec<String> = hits.iter().map(|r| r.platform.clone()).collect();
                have.sort_unstable();
                have.dedup();
                Err(Error::NoSuchImage(format!(
                    "{want} for {prefer}. The store holds it for {}",
                    have.join(", ")
                )))
            }
            0 if hits.len() == 1 => Ok(hits.into_iter().next().expect("length checked")),
            _ => {
                let mut have: Vec<String> = hits.iter().map(|r| r.platform.clone()).collect();
                have.sort_unstable();
                have.dedup();
                Err(Error::Usage(format!(
                    "{want} names {} images in this store, for {}. podvm will \
                     not pick one by position: name the platform with \
                     --platform, or the image by its digest",
                    hits.len(),
                    have.join(", ")
                )))
            }
        }
    }

    // ------------------------------------------------------------ the writes

    /// Record a pull. Replaces the record for the same repository, tag **and
    /// platform**, because a moving tag is the normal case.
    ///
    /// ⛔ **The platform is part of the key and that is a multi-architecture
    /// decision.** `alpine:latest` for `linux/amd64` and for `linux/arm64` are
    /// two different images that share one name, and keying without the
    /// platform means the second pull silently deletes the first: the record
    /// goes, the blobs are collected, and a caller who pulled both has one.
    /// podman keeps both and so does podvm. `Store::find` is where the
    /// resulting ambiguity is resolved, by name and never by position.
    pub fn put_record(&self, record: Record) -> Result<()> {
        let _guard = self.lock()?;
        let mut index = self.read_index()?;
        index.podvm_store = INDEX_VERSION;
        index.images.retain(|r| {
            !(r.repository == record.repository
                && r.tag == record.tag
                && r.platform == record.platform
                && (record.tag.is_some() || r.digest == record.digest))
        });
        index.images.push(record);
        self.write_index(&index)
    }

    /// `podvm tag <src> <dst>`. The destination points at the same manifest
    /// digest; nothing is fetched and no blob is copied.
    pub fn tag(&self, src: &str, dst: &str) -> Result<Record> {
        let source = self.find_one(src)?;
        let target = Reference::parse(dst)?;
        if target.digest.is_some() {
            return Err(Error::Usage(format!(
                "{dst:?} names a digest. A tag is a name podvm assigns, and a \
                 digest is one the content assigns; the second cannot be set"
            )));
        }
        let record = Record {
            repository: target.canonical_repository(),
            tag: target.tag.clone(),
            pulled_at: clock::now(),
            ..source
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    /// `podvm rmi`. Removes the records a reference names and then every blob
    /// no surviving record reaches.
    ///
    /// ⛔ Refuses while a container holds the image, and says which.
    pub fn remove(&self, want: &str) -> Result<Removed> {
        let doomed = self.find(want)?;
        if doomed.is_empty() {
            return Err(Error::NoSuchImage(want.to_string()));
        }
        // ⛔ Invariant I4: the `in_use` check is inside `delete`, under the
        // index lock, and never here. This function used to make it and it was
        // stale by the time `delete` acted on it.
        self.delete(&doomed, Held::Refuse)
    }

    /// `podvm image prune`. Without `all`, only untagged records; with it,
    /// every record no container holds.
    pub fn prune(&self, all: bool) -> Result<Removed> {
        let doomed = self.prune_candidates(all)?;
        // ⛔ Invariant I4: what is held is decided under the lock, by `delete`,
        // and `Held::Skip` is what makes a held image a named skip here where
        // `rmi` refuses outright.
        self.delete(&doomed, Held::Skip)
    }

    /// The records `prune` would consider, before any hold or reference
    /// check. One home for the untagged/`all` filter, so the CLI's
    /// record-gate (TODO/image.md T-1322) and `prune` cannot disagree
    /// about the candidate set.
    pub fn prune_candidates(&self, all: bool) -> Result<Vec<Record>> {
        let mut doomed = Vec::new();
        for r in self.list()? {
            if !all && r.tag.is_some() {
                continue;
            }
            doomed.push(r);
        }
        Ok(doomed)
    }

    /// Delete records with the hold check under the index lock.
    ///
    /// ⛔ INVARIANT I4 lives here: `in_use` is decided under the same lock
    /// that guards the unlinks below, so a `hold` cannot be taken between
    /// the answer and the act. What this does NOT see is container
    /// records: `create` takes no hold, so a created container's
    /// reference is invisible to the lock. Callers that delete on behalf
    /// of a verb (TODO/image.md T-1322) gate records first with
    /// `podvm_supervise::referencing` and pass only the unreferenced
    /// remainder here; the hold check below stays the last word on
    /// running payloads either way.
    pub fn delete(&self, doomed: &[Record], on_held: Held) -> Result<Removed> {
        let _guard = self.lock()?;
        // ⛔ INVARIANT I4. Inside the lock, so a `hold` cannot be taken between
        // this answer and the unlinks below: `Store::hold` takes the same lock.
        let mut skipped = Vec::new();
        let mut kept = Vec::new();
        for r in doomed {
            if self.in_use(r)? {
                match on_held {
                    Held::Refuse => {
                        return Err(Error::Store(format!(
                            "{} is in use by a running container and was not removed. \
                             A GC that deletes an extraction out from under a payload \
                             makes its failure read as a missing file rather than as a \
                             concurrent deletion (TODO/image.md T-0204)",
                            r.name()
                        )))
                    }
                    Held::Skip => {
                        skipped.push(r.name());
                        continue;
                    }
                }
            }
            kept.push(r.clone());
        }
        let doomed: &[Record] = &kept;
        let mut index = self.read_index()?;
        let doomed_keys: Vec<(String, Option<String>, String)> = doomed
            .iter()
            .map(|r| (r.repository.clone(), r.tag.clone(), r.digest.clone()))
            .collect();
        index.images.retain(|r| {
            !doomed_keys
                .iter()
                .any(|(repo, tag, dig)| r.repository == *repo && r.tag == *tag && r.digest == *dig)
        });

        // ⭐ Reachability over what SURVIVES, not over what was deleted. A blob
        // several tags share is kept while any of them remains, and computing
        // the doomed set instead would delete it with the first.
        let mut keep: Vec<String> = Vec::new();
        for r in &index.images {
            keep.extend(r.blobs().into_iter().map(str::to_string));
        }
        let mut freed_bytes = 0u64;
        let mut freed = Vec::new();
        for r in doomed {
            for b in r.blobs() {
                if keep.iter().any(|k| k == b) || freed.iter().any(|f| f == b) {
                    continue;
                }
                let d = Digest::parse(b)?;
                let path = self.blob_path(&d);
                // ⛔ One gate per action: every unlink this crate performs is
                // resolved against the store root first.
                let resolved = contain::within(&self.root, &path)?;
                if let Ok(meta) = std::fs::metadata(&resolved) {
                    freed_bytes += meta.len();
                }
                match std::fs::remove_file(&resolved) {
                    Ok(()) => freed.push(b.to_string()),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => freed.push(b.to_string()),
                    Err(e) => return Err(Error::io(resolved.display().to_string(), e)),
                }
            }
            let lock = self.image_lock_path(r)?;
            if lock.exists() {
                let resolved = contain::within(&self.root, &lock)?;
                let _ = std::fs::remove_file(resolved);
            }
        }
        self.write_index(&index)?;
        Ok(Removed {
            untagged: doomed.iter().map(Record::name).collect(),
            deleted: freed,
            freed_bytes,
            skipped,
        })
    }
}
