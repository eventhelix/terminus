// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 EventHelix.com Inc.

//! Library surface of the `terminus` binary — exposed so integration
//! tests (determinism, golden, smoke) can run scenarios in-process.

pub mod assemble;
pub mod config;
pub mod output;
