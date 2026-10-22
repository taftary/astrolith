//! Ground memory limit of the settings (`E-FILE-SIZE` split from
//! `settings`, #918 S1). Re-exported from [`crate::settings`].

/// Most memory the ground patches may use (#918 S1, P2 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GroundMemory {
    /// 32 MB: smaller phones; far ground coarser.
    Mb32,
    /// 64 MB (default).
    #[default]
    Mb64,
    /// 128 MB: more detail toward the horizon.
    Mb128,
}

impl GroundMemory {
    /// Every choice in cycle order.
    pub const ALL: [GroundMemory; 3] =
        [GroundMemory::Mb32, GroundMemory::Mb64, GroundMemory::Mb128];

    /// Next choice when the setting is clicked.
    #[must_use]
    pub fn next(self) -> GroundMemory {
        match self {
            GroundMemory::Mb32 => GroundMemory::Mb64,
            GroundMemory::Mb64 => GroundMemory::Mb128,
            GroundMemory::Mb128 => GroundMemory::Mb32,
        }
    }

    /// The limit in megabytes.
    #[must_use]
    pub fn megabytes(self) -> usize {
        match self {
            GroundMemory::Mb32 => 32,
            GroundMemory::Mb64 => 64,
            GroundMemory::Mb128 => 128,
        }
    }

    /// The limit in bytes.
    #[must_use]
    pub fn bytes(self) -> usize {
        self.megabytes() << 20
    }

    /// File id (`ground_memory_limit = <id>`).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            GroundMemory::Mb32 => "32",
            GroundMemory::Mb64 => "64",
            GroundMemory::Mb128 => "128",
        }
    }

    /// Display text shown in Settings.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            GroundMemory::Mb32 => "32 MB",
            GroundMemory::Mb64 => "64 MB",
            GroundMemory::Mb128 => "128 MB",
        }
    }

    /// Choice from a file id, or `None` for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Option<GroundMemory> {
        GroundMemory::ALL
            .into_iter()
            .find(|choice| choice.id() == id)
    }
}
