+++
id = "I-004"
title = "security: peregrin-dotenv writes secrets file world-readable + accepts newline/= injection"
status = "open"
created = "2026-11-03"
priority = "medium"
+++

DotenvSecrets::set uses std::fs::write (umask default ~0644) with no atomic write and no validation — names/values containing '=' or newlines corrupt/inject entries. Fix: validate name/value, write 0600 via tmp+rename.
