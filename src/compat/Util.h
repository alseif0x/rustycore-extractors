// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// Standalone replacement for the broad TrinityCore Util dependency.

#pragma once

#include "Define.h"
#include <string>

enum class TimeFormat
{
    FullText,
    ShortText,
    Numeric
};

namespace Trinity
{
void VerifyOsVersion();
}

std::string secsToTimeString(uint64 timeInSecs, TimeFormat timeFormat = TimeFormat::FullText, bool hoursOnly = false);
