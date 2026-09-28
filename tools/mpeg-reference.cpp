// Standalone oracle for the pinned MAME Layer II decoder, NOT linked to TGPulse.
// Build/use and reference limitations: docs/MODEL1_DSB.md.
#include <cstdint>
#include "mpeg_audio.h"
#include <fstream>
#include <iomanip>
#include <iostream>
#include <iterator>
#include <vector>
#include <climits>
int main(int argc, char **argv) {
    if (argc != 2) return 2;
    std::ifstream in(argv[1], std::ios::binary);
    if (!in) return 2;
    std::vector<unsigned char> bytes((std::istreambuf_iterator<char>(in)), {});
    if (bytes.size() > INT_MAX/8) return 2;
    int limit = int(bytes.size()*8);
    // Guard the reference's bit reader; guards are outside the reported limit.
    bytes.resize(bytes.size()+16, 0);
    mpeg_audio decoder(bytes.data(), mpeg_audio::L2, false, 0);
    int pos=0, frames=0;
    int16_t pcm[2304];
    int count, rate, channels;
    while (decoder.decode_buffer(pos,limit,pcm,count,rate,channels)) {
        uint64_t hash=0xcbf29ce484222325ULL;
        for (int i=0;i<count*channels;i++) {
            uint16_t v=uint16_t(pcm[i]);
            hash=(hash^(v&255))*0x100000001b3ULL;
            hash=(hash^(v>>8))*0x100000001b3ULL;
        }
        std::cout << pos << " " << channels << " " << rate << " " << count << " "
                  << std::hex << std::setw(16) << std::setfill('0') << hash << std::dec << "\n";
        if (++frames >= 10000) return 3;
    }
}
