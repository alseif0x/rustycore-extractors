// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// Standalone structured-diagnostic adapter.

#pragma once

#include <fmt/format.h>
#include <cstdio>
#include <utility>

namespace RustyCore::Diagnostics
{
template <typename... Args>
void Write(char const* level, char const* channel, fmt::format_string<Args...> message, Args&&... args)
{
    std::string text = fmt::format(message, std::forward<Args>(args)...);
    std::fprintf(stderr, "[%s] [%s] %s\n", level, channel ? channel : "", text.c_str());
}
}

#define TC_LOG_ERROR(channel, ...) ::RustyCore::Diagnostics::Write("error", channel, __VA_ARGS__)
#define TC_LOG_DEBUG(channel, ...) ::RustyCore::Diagnostics::Write("debug", channel, __VA_ARGS__)
