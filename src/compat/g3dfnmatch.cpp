// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// Fresh portable glob matcher; no code is copied from the excluded G3D files.

#include "G3D/g3dfnmatch.h"
#include <cctype>
#include <cstring>

namespace
{
char Fold(char value, int flags)
{
    return (flags & FNM_CASEFOLD) ? static_cast<char>(std::tolower(static_cast<unsigned char>(value))) : value;
}

bool IsLeadingPeriod(char const* begin, char const* current, int flags)
{
    return *current == '.' && (flags & FNM_PERIOD) &&
        (current == begin || ((flags & FNM_PATHNAME) && current[-1] == '/'));
}

bool MatchClass(char const*& pattern, char value, int flags)
{
    bool negate = *pattern == '!' || *pattern == '^';
    if (negate)
        ++pattern;

    bool matched = false;
    char test = Fold(value, flags);
    while (*pattern && *pattern != ']')
    {
        char first = *pattern++;
        if (first == '\\' && !(flags & FNM_NOESCAPE) && *pattern)
            first = *pattern++;
        first = Fold(first, flags);

        if (*pattern == '-' && pattern[1] && pattern[1] != ']')
        {
            ++pattern;
            char last = *pattern++;
            if (last == '\\' && !(flags & FNM_NOESCAPE) && *pattern)
                last = *pattern++;
            last = Fold(last, flags);
            matched = matched || (first <= test && test <= last);
        }
        else
            matched = matched || first == test;
    }

    if (*pattern != ']')
        return false;
    ++pattern;
    return negate ? !matched : matched;
}

bool Match(char const* pattern, char const* string, char const* stringBegin, int flags)
{
    while (*pattern)
    {
        if (*pattern == '*')
        {
            while (*pattern == '*')
                ++pattern;
            if (IsLeadingPeriod(stringBegin, string, flags))
                return false;
            if (!*pattern)
                return !(flags & FNM_PATHNAME) || !std::strchr(string, '/') || (flags & FNM_LEADING_DIR);
            for (char const* candidate = string;; ++candidate)
            {
                if (Match(pattern, candidate, stringBegin, flags))
                    return true;
                if (!*candidate || ((flags & FNM_PATHNAME) && *candidate == '/'))
                    return false;
            }
        }

        if (!*string)
            return false;
        if ((flags & FNM_PATHNAME) && *string == '/' && *pattern != '/')
            return false;
        if (IsLeadingPeriod(stringBegin, string, flags) && *pattern != '.')
            return false;

        if (*pattern == '?')
        {
            ++pattern;
            ++string;
            continue;
        }

        if (*pattern == '[')
        {
            char const* classPattern = pattern + 1;
            if (!MatchClass(classPattern, *string, flags))
                return false;
            pattern = classPattern;
            ++string;
            continue;
        }

        char expected = *pattern++;
        if (expected == '\\' && !(flags & FNM_NOESCAPE) && *pattern)
            expected = *pattern++;
        if (Fold(expected, flags) != Fold(*string, flags))
            return false;
        ++string;
    }

    return !*string || ((flags & FNM_LEADING_DIR) && *string == '/');
}
}

int G3D::g3dfnmatch(char const* pattern, char const* string, int flags)
{
    if (!pattern || !string)
        return FNM_NOMATCH;
    return Match(pattern, string, string, flags) ? 0 : FNM_NOMATCH;
}
