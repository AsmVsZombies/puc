// survive calculator dispatch — its own TU. Like calc_pos.cpp it spawns 5x of
// every requested zombie type and runs each wave to wave_length, but it measures
// HP-based "受击" (hit) instead of x-position: a zombie is hit iff it is dead OR
// took >= hitThres damage. HP follows the refresh accounting
// (body + accessory_1 + accessory_2 * 0.2; see lib/system/spawn.cpp get_current_hp).
//
// It reuses pos::load_wave (defined in calc_pos.cpp's TU) via a forward
// declaration so this TU need not include the non-inline seml/pos/operation.h,
// which would collide at link time.

#include <map>
#include <mutex>
#include <thread>
#include <utility>
#include <vector>

#include "rapidjson/document.h"
#include "rapidjson/stringbuffer.h"
#include "rapidjson/writer.h"

#include "common/pe.h"      // ::run
#include "common/test.h"    // assign_repeat
#include "world.h"

#include "seml/pos/types.h"    // pos::Test
#include "seml/reader/lib.h"   // read_json_string, Config, Setting, Wave

#include "params.h"

// Defined in calc_pos.cpp's TU. Assembles a wave (setup/spawn 5x each/ice/cobs/
// cards/fodder) into a pos::Test.
namespace pos {
void load_wave(const Setting& setting, const Wave& wave,
    const std::vector<pvz_emulator::object::zombie_type>& zombie_types, bool huge, Test& test);
}

namespace {

using zombie_type = pvz_emulator::object::zombie_type;

// Matches the literal constant in lib/system/spawn.cpp get_current_hp().
constexpr double ACC2_WEIGHT = 0.2000000029802322;

// Weighted hp = body + accessory_1 + (unsigned)(accessory_2 * 0.2) + balloon 20,
// matching lib/system/spawn.cpp get_current_hp. Uses live `.hp` fields (not
// `.max_hp`, which the emulator zeroes when an accessory is destroyed); the
// initial baseline is snapshotted at spawn instead. Body is clamped at 0 so a
// dying zombie's transient negative hp can't go below zero.
inline double weighted_hp(const pvz_emulator::object::zombie& z)
{
    int body = z.hp < 0 ? 0 : z.hp;
    return static_cast<double>(body) + static_cast<double>(z.accessory_1.hp)
        + static_cast<double>(static_cast<unsigned>(z.accessory_2.hp * ACC2_WEIGHT))
        + (z.has_balloon ? 20.0 : 0.0);
}

struct SurviveStats {
    int total_count = 0;    // 5 per (wave, type) per repeat
    int not_hit_count = 0;  // alive AND damage < hitThres
    double hit_hp_sum = 0.0;     // sum of current hp over alive-but-hit zombies
    double not_hit_hp_sum = 0.0; // sum of current hp over not-hit zombies

    void merge(const SurviveStats& o)
    {
        total_count += o.total_count;
        not_hit_count += o.not_hit_count;
        hit_hp_sum += o.hit_hp_sum;
        not_hit_hp_sum += o.not_hit_hp_sum;
    }
};

using StatsKey = std::pair<int, zombie_type>; // (wave_idx, type)
using StatsMap = std::map<StatsKey, SurviveStats>;

void test_one(const Config& config, int repeat,
    const std::vector<zombie_type>& zombie_types, bool disable_cob_delay, bool huge, int hit_thres,
    std::mutex& mtx, StatsMap& out)
{
    using namespace pvz_emulator;
    using namespace pvz_emulator::object;

    world w(config.setting.scene_type);
    StatsMap local;

    for (int r = 0; r < repeat; r++) {
        for (size_t wave_idx = 0; wave_idx < config.waves.size(); wave_idx++) {
            const auto& wave = config.waves[wave_idx];
            pos::Test test;
            pos::load_wave(config.setting, wave, zombie_types, huge, test);

            w.scene.reset();
            w.scene.stop_spawn = true;
            w.scene.ignore_game_over = true;
            w.scene.disable_cob_delay = disable_cob_delay;

            // Run setup + spawn (everything at tick <= 0), then snapshot each
            // zombie's initial weighted hp (full accessories, balloon intact).
            auto it = test.ops.begin();
            int curr_tick = it->tick;
            for (; it != test.ops.end() && it->tick <= 0; it++) {
                ::run(w, curr_tick, it->tick);
                it->f(w);
            }
            ::run(w, curr_tick, 0);

            std::map<int, double> init_hp; // zombie uuid -> initial weighted hp
            for (const auto& z : w.scene.zombies) {
                if (z.master_id == -1) {
                    init_hp[z.uuid] = weighted_hp(z);
                }
            }

            // Remaining ops (cobs/cards/fodder), then settle to wave_length.
            for (; it != test.ops.end() && it->tick <= wave.wave_length; it++) {
                ::run(w, curr_tick, it->tick);
                it->f(w);
            }
            ::run(w, curr_tick, wave.wave_length);

            for (const auto& type : zombie_types) {
                local[{static_cast<int>(wave_idx), type}].total_count += 5;
            }

            // Only ALIVE zombies are inspected here. The hit_count is derived as
            // total - not_hit so that dead zombies (possibly already removed from
            // the list) are all accounted for and contribute 0 hp to the average.
            for (const auto& z : w.scene.zombies) {
                // Judge aliveness exactly as the refresh calculator does
                // (lib/system/spawn.cpp get_current_hp / refresh zombie_count):
                // skip hypnotized zombies, any death animation, and summons.
                // has_death_status() covers dying_from_instant_kill, which an ash
                // attack (cob/jalapeno) uses for flying/special zombies (balloon/
                // yeti/bungee) — they keep full hp for 300cs before vanishing, so
                // without this they would read as survivors.
                if (z.is_hypno || z.has_death_status() || z.master_id != -1)
                    continue;
                bool requested = false;
                for (const auto& type : zombie_types) {
                    if (z.type == type) {
                        requested = true;
                        break;
                    }
                }
                if (!requested)
                    continue;

                double curr = weighted_hp(z);
                auto found = init_hp.find(z.uuid);
                double init = (found != init_hp.end()) ? found->second : curr;

                auto& s = local[{static_cast<int>(wave_idx), z.type}];
                if (init - curr < hit_thres) {
                    s.not_hit_count++;
                    s.not_hit_hp_sum += curr;
                } else {
                    s.hit_hp_sum += curr; // alive but heavily damaged => hit
                }
            }
        }
    }

    std::lock_guard<std::mutex> guard(mtx);
    for (const auto& [k, v] : local) {
        out[k].merge(v);
    }
}

} // namespace

std::string puc_dispatch_survive(const char* scenario_json, const char* params_json)
{
    Config config = read_json_string(scenario_json);
    auto params = puc_params::parse(params_json);

    std::vector<zombie_type> zombie_types;
    for (int n : puc_params::opt_int_array(params, "zombies")) {
        zombie_types.push_back(static_cast<zombie_type>(n));
    }
    int repeat = puc_params::opt_int(params, "repeat", 20000);
    bool disable_cob_delay = puc_params::opt_bool(params, "disableCobDelay", true);
    bool huge = puc_params::opt_bool(params, "huge", false);
    int hit_thres = puc_params::opt_int(params, "hitThres", 1800);
    unsigned thread_num = puc_params::opt_uint(params, "threadNum", 0);

    unsigned n = thread_num ? thread_num : std::thread::hardware_concurrency();
    if (n == 0)
        n = 1;

    std::mutex mtx;
    StatsMap merged;
    std::vector<std::thread> threads;
    for (int rep : assign_repeat(repeat, static_cast<int>(n))) {
        threads.emplace_back([&, rep] {
            test_one(config, rep, zombie_types, disable_cob_delay, huge, hit_thres, mtx, merged);
        });
    }
    for (auto& t : threads) {
        t.join();
    }

    struct Column {
        int wave_idx;
        int wave_length;
        zombie_type type;
    };
    std::vector<Column> columns;
    for (size_t wi = 0; wi < config.waves.size(); wi++) {
        for (const auto& type : zombie_types) {
            columns.push_back({static_cast<int>(wi), config.waves[wi].wave_length, type});
        }
    }

    rapidjson::StringBuffer sb;
    rapidjson::Writer<rapidjson::StringBuffer> w(sb);

    w.StartObject();
    w.Key("calculator");
    w.String("survive");
    w.Key("repeat");
    w.Int(repeat);
    w.Key("hitThres");
    w.Int(hit_thres);

    w.Key("columns");
    w.StartArray();
    for (const auto& c : columns) {
        w.StartObject();
        w.Key("waveIdx");
        w.Int(c.wave_idx);
        w.Key("waveLength");
        w.Int(c.wave_length);
        w.Key("zombieType");
        w.Int(static_cast<int>(c.type));
        w.EndObject();
    }
    w.EndArray();

    static const SurviveStats empty;
    w.Key("stats");
    w.StartArray();
    for (const auto& c : columns) {
        auto it = merged.find({c.wave_idx, c.type});
        const SurviveStats& s = (it != merged.end()) ? it->second : empty;
        w.StartObject();
        w.Key("totalCount");
        w.Int(s.total_count);
        w.Key("hitCount");
        w.Int(s.total_count - s.not_hit_count);
        w.Key("hitHpSum");
        w.Double(s.hit_hp_sum);
        w.Key("notHitHpSum");
        w.Double(s.not_hit_hp_sum);
        w.EndObject();
    }
    w.EndArray();
    w.EndObject();

    return sb.GetString();
}
