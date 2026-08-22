// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "Util.h"
#include "Locales.h"
#include <iomanip>
#include <locale>
#include <sstream>

namespace
{
std::locale GlobalLocale = std::locale::classic();
}

void Trinity::VerifyOsVersion()
{
    // The standalone build relies on the compiler/runtime support matrix.
}

void Trinity::Locale::Init()
{
    GlobalLocale = std::locale::classic();
    std::locale::global(GlobalLocale);
}

std::locale const& Trinity::Locale::GetGlobalLocale()
{
    return GlobalLocale;
}

std::locale const& Trinity::Locale::GetCalendarLocale()
{
    return GlobalLocale;
}

std::string secsToTimeString(uint64 timeInSecs, TimeFormat timeFormat, bool hoursOnly)
{
    uint64 days = timeInSecs / 86400;
    uint64 hours = (timeInSecs / 3600) % 24;
    uint64 minutes = (timeInSecs / 60) % 60;
    uint64 seconds = timeInSecs % 60;

    std::ostringstream output;
    if (timeFormat == TimeFormat::Numeric)
    {
        if (days)
            output << days << ':' << std::setfill('0') << std::setw(2);
        output << (days ? hours : timeInSecs / 3600) << ':' << std::setfill('0') << std::setw(2)
               << minutes << ':' << std::setw(2) << seconds;
        return output.str();
    }

    if (days)
        output << days << (timeFormat == TimeFormat::ShortText ? "d " : " day(s) ");
    if (days || hours || hoursOnly)
        output << hours << (timeFormat == TimeFormat::ShortText ? "h " : " hour(s) ");
    if (!hoursOnly)
    {
        output << minutes << (timeFormat == TimeFormat::ShortText ? "m " : " minute(s) ");
        output << seconds << (timeFormat == TimeFormat::ShortText ? "s" : " second(s)");
    }
    return output.str();
}
