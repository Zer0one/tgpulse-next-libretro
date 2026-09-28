// Standalone control-boundary oracle; NOT a production dependency.
// Build/replay instructions and pinned revision: docs/MODEL1_AUDIO.md.
// Reads W port data / A clocks / R reset / P power-on / F seed count.
// --audio hashes generated frames (no state inspection); T pattern channel
// starts a synthetic tone on a fresh chip, U adds a tone, N advances frames.
// Trailing # hash is ignored, so the checked-in trace can be regenerated.
#include "ymfm_opn.h"
#include <algorithm>
#include <cassert>
#include <cstdint>
#include <iomanip>
#include <iostream>
#include <limits>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

struct Host : ymfm::ymfm_interface {
    static constexpr uint64_t NEVER = std::numeric_limits<uint64_t>::max();
    uint64_t now = 0, busy_end = 0, next_sample = 144;
    uint64_t deadline[2] = {NEVER, NEVER};
    bool irq = false;
    bool capture = false;
    std::vector<int32_t> samples;
    void ymfm_set_timer(uint32_t n, int32_t clocks) override {
        deadline[n] = clocks < 0 ? NEVER : now + clocks;
    }
    void ymfm_set_busy_end(uint32_t clocks) override { busy_end = now + clocks; }
    bool ymfm_is_busy() override { return now < busy_end; }
    void ymfm_update_irq(bool value) override { irq = value; }
    void advance(ymfm::ym3438 &chip, uint32_t clocks) {
        const uint64_t end = now + clocks;
        for (;;) {
            uint64_t event = std::min(next_sample, std::min(deadline[0], deadline[1]));
            if (event > end) break;
            now = event;
            // Explicit shared contract at ties: timer A, timer B, sample, bus.
            for (unsigned n = 0; n < 2; ++n)
                if (deadline[n] == now) {
                    deadline[n] = NEVER;
                    m_engine->engine_timer_expired(n);
                }
            if (next_sample == now) {
                ymfm::ym3438::output_data discarded;
                chip.generate(&discarded);
                if (capture) {
                    samples.push_back(discarded.data[0]);
                    samples.push_back(discarded.data[1]);
                }
                next_sample += 144;
            }
        }
        now = end;
    }
};

struct Chip : ymfm::ym3438 {
    explicit Chip(Host &host) : ymfm::ym3438(host) { reset(); }
    uint8_t reg(unsigned n) { return m_fm.regs().read(n); }
    uint16_t address() { return m_address; }
    uint16_t dac() { return m_dac_data; }
    uint8_t dac_enabled() { return m_dac_enable; }
};

struct Hash {
    uint64_t value = UINT64_C(14695981039346656037);
    void add(uint64_t data, unsigned bytes) {
        for (unsigned i = 0; i < bytes; ++i) {
            value = (value ^ (data & 255)) * UINT64_C(1099511628211);
            data >>= 8;
        }
    }
};

struct Rig {
    Host host;
    Chip chip;
    Rig() : chip(host) {}
    void reg(uint16_t address, uint8_t value) {
        unsigned bank = (address >> 8) * 2;
        chip.write(bank, address & 255);
        chip.write(bank + 1, value);
    }
    void tone(uint32_t pattern, unsigned channel) {
        assert(channel < 6);
        unsigned ch = channel % 3 + 0x100 * (channel / 3);
        reg(0xb0 + ch, pattern & 63);
        const uint8_t pans[] = {0xc0, 0x80, 0x40};
        reg(0xb4 + ch, pans[channel % 3] | (((pattern >> 14) & 3) << 4) | ((pattern >> 16) & 7));
        reg(0x22, (pattern >> 10) & 15);
        const unsigned offsets[] = {0, 8, 4, 12};
        const uint8_t dtmul[] = {0x01, 0x32, 0x73, 0x40};
        const uint8_t tl[] = {12, 24, 32, 16};
        const uint8_t ar[] = {0x1f, 0x54, 0x98, 0xdf};
        const uint8_t dr[] = {18, 12, 24, 16};
        const uint8_t sr[] = {6, 10, 8, 4};
        const uint8_t slrr[] = {0x28, 0x49, 0x3a, 0x6f};
        for (unsigned slot = 0; slot < 4; ++slot) {
            unsigned op = ch + offsets[slot];
            reg(0x30 + op, dtmul[slot]); reg(0x40 + op, tl[slot]);
            reg(0x50 + op, ar[slot]); reg(0x60 + op, 0x80 | dr[slot]);
            reg(0x70 + op, sr[slot]); reg(0x80 + op, slrr[slot]);
            reg(0x90 + op, (pattern >> 6) & 15);
        }
        reg(0xa4 + ch, 0x22); reg(0xa0 + ch, 0x69 + 7 * channel);
        for (unsigned i = 0; i < 3; ++i) {
            reg(0xac + i, 0x18 + i * 4); reg(0xa8 + i, 0x47 + i * 37);
        }
        const unsigned mode = (pattern >> 19) & 3;
        reg(0x24, 240); reg(0x25, 2); reg(0x27, (mode << 6) | (mode == 2 ? 1 : 0));
        reg(0x28, (channel % 3) | ((channel / 3) << 2) | (mode == 2 ? 0 : 0xf0));
    }
    uint64_t audio_hash() {
        Hash result;
        result.add(host.samples.size() / 2, 4);
        for (int32_t sample : host.samples) result.add(uint32_t(sample), 4);
        host.samples.clear();
        return result.value;
    }
    uint64_t hash() {
        Hash hash;
        for (unsigned n = 0; n < 512; ++n) hash.add(chip.reg(n), 1);
        hash.add(chip.address(), 2);
        hash.add(chip.dac(), 2);
        hash.add(chip.dac_enabled(), 1);
        for (unsigned port = 0; port < 4; ++port) hash.add(chip.read(port), 1);
        hash.add(host.irq, 1);
        hash.add(host.busy_end > host.now ? host.busy_end - host.now : 0, 2);
        for (unsigned n = 0; n < 2; ++n)
            hash.add(host.deadline[n] == Host::NEVER ? 0 : host.deadline[n] - host.now, 4);
        hash.add(host.next_sample - host.now, 2);
        // Pinned YMFM state layout only (not a public/stable serialization ABI).
        // 5 interface bytes + 11 engine bytes + 517 register bytes + 36 channel
        // bytes + 24 ten-byte operators = 809. Last operator byte is keyon_live.
        std::vector<uint8_t> snapshot;
        ymfm::ymfm_saved_state save(snapshot, true);
        chip.save_restore(save);
        assert(snapshot.size() == 809);
        hash.add(snapshot[15], 1); // low 8-bit engine sample counter
        const unsigned offsets[4] = {0, 6, 3, 9};
        bool csm = false;
        for (unsigned channel = 0; channel < 6; ++channel) {
            unsigned keys = 0;
            for (unsigned slot = 0; slot < 4; ++slot) {
                unsigned op = channel % 3 + (channel / 3) * 12 + offsets[slot];
                uint8_t live = snapshot[569 + op * 10 + 9];
                keys |= (live & 1) << slot;
                if (channel == 2) csm |= (live & 4) != 0;
            }
            hash.add(keys, 1);
        }
        hash.add(csm, 1);
        return hash.value;
    }
    uint64_t fuzz(uint32_t seed, unsigned count) {
        Hash history;
        const uint16_t regs[] = {0x24,0x25,0x26,0x27,0x28,0x2a,0x2b,0x2c,
            0xa0,0xa4,0xa8,0xac,0xb8,0x124,0x128,0x1a1,0x1a5,0x1b6};
        for (unsigned i = 0; i < count; ++i) {
            seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5;
            switch (seed & 7) {
                case 0: case 1: case 2:
                    host.advance(chip, 1 + ((seed >> 8) & 4095)); break;
                case 3: case 4:
                    chip.write((seed >> 8) & 3, seed >> 16); break;
                default:
                    reg(regs[(seed >> 8) % 18], seed >> 24); break;
            }
            history.add(hash(), 8); // compare every step, not just the final state
        }
        return history.value;
    }
};

int main(int argc, char **argv) {
    bool audio = argc == 2 && std::string(argv[1]) == "--audio";
    auto rig = std::make_unique<Rig>();
    rig->host.capture = audio;
    std::string line;
    while (std::getline(std::cin, line)) {
        line = line.substr(0, line.find('#'));
        std::istringstream input(line);
        char op;
        if (!(input >> op)) continue;
        uint32_t a = 0, b = 0;
        input >> a >> b;
        uint64_t hash = 0;
        switch (op) {
            case 'P': rig = std::make_unique<Rig>(); rig->host.capture = audio; break;
            case 'R': rig->chip.reset(); break;
            case 'A': rig->host.advance(rig->chip, a); break;
            case 'W': rig->chip.write(a, b); break;
            case 'F': hash = rig->fuzz(a, b); break;
            case 'T': rig = std::make_unique<Rig>(); rig->host.capture = audio; rig->tone(a, b); break;
            case 'U': rig->tone(a, b); break;
            case 'N': rig->host.advance(rig->chip, a * 144); break;
            default: std::cerr << "unknown command\n"; return 1;
        }
        if (audio) hash = rig->audio_hash();
        else if (op != 'F') hash = rig->hash();
        std::cout << op;
        if (op == 'A' || op == 'W' || op == 'F' || op == 'T' || op == 'U' || op == 'N') std::cout << ' ' << std::dec << a;
        if (op == 'W' || op == 'F' || op == 'T' || op == 'U') std::cout << ' ' << std::dec << b;
        std::cout << " # " << std::hex << std::setfill('0') << std::setw(16) << hash << '\n';
    }
}
