// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded bitset/wire QA, no client input.
// Original MapDefines.h/.cpp are externally pinned to TrinityCore
// 02245dcd245e7433e524577656177723d3e4992e with notices retained by the builder.
// Source System.cpp:1135-1144 writes the original magic/version/build followed
// by std::bitset<4096>::to_string(). This driver independently exercises that
// container/serialization behavior, not native WDT/CASC discovery or ConvertADT.
#include "MapDefines.h"
#include <bit>
#include <bitset>
#include <cstdint>
#include <cstring>
#include <iostream>
int main() {
    static_assert(std::endian::native == std::endian::little);
    char magic[8];std::uint32_t build,count;
    if(!std::cin.read(magic,8)||std::memcmp(magic,"RCEBTL01",8)!=0
       ||!std::cin.read(reinterpret_cast<char*>(&build),4)
       ||!std::cin.read(reinterpret_cast<char*>(&count),4)
       ||(build!=70170&&build!=70205)||count>4096) return 4;
    std::bitset<4096> existing;
    for(std::uint32_t i=0;i<count;++i) {
        unsigned char xy[2];if(!std::cin.read(reinterpret_cast<char*>(xy),2)||xy[0]>=64||xy[1]>=64)return 4;
        existing[std::size_t(xy[1])*64+xy[0]]=true;
    }
    if(std::cin.peek()!=std::char_traits<char>::eof())return 4;
    std::cout.write(MapMagic.data(),MapMagic.size());
    std::cout.write(reinterpret_cast<char const*>(&MapVersionMagic),sizeof(MapVersionMagic));
    std::cout.write(reinterpret_cast<char const*>(&build),4);
    auto bits=existing.to_string();std::cout.write(bits.data(),bits.size());
    return std::cout.good()?0:4;
}
