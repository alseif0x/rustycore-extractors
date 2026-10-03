// SPDX-License-Identifier: GPL-3.0-or-later
// Modern API orders Final(context, output), unlike the legacy adapter.
#pragma once
using u32 = unsigned int;
struct evp_md_ctx_st;
struct SHA1_CTX { evp_md_ctx_st* context; };
void SHA1_Init(SHA1_CTX*);
void SHA1_Update(SHA1_CTX*, void const*, u32);
void SHA1_Final(SHA1_CTX*, unsigned char[20]);
