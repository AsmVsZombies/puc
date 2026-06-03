// refresh calculator dispatch — see calc_pos.cpp for the per-TU rationale.

#include "rapidjson/document.h"

#include "seml/refresh/run.h"

#include "params.h"

namespace {

refresh::ZombieTypes to_zombie_set(const std::vector<int>& ints)
{
    refresh::ZombieTypes set;
    for (int n : ints) {
        set.insert(static_cast<pvz_emulator::object::zombie_type>(n));
    }
    return set;
}

} // namespace

std::string puc_dispatch_refresh(const char* scenario_json, const char* params_json)
{
    Config config = read_json_string(scenario_json);
    auto params = puc_params::parse(params_json);

    refresh::RunParams p;
    p.required_types = to_zombie_set(puc_params::opt_int_array(params, "require"));
    p.banned_types = to_zombie_set(puc_params::opt_int_array(params, "ban"));
    p.huge = puc_params::opt_bool(params, "huge", p.huge);
    p.assume_activate = puc_params::opt_bool(params, "activate", p.assume_activate);
    p.use_dance_cheat = puc_params::opt_bool(params, "dance", p.use_dance_cheat);
    p.natural = puc_params::opt_bool(params, "natural", p.natural);
    p.disable_cob_delay = puc_params::opt_bool(params, "disableCobDelay", p.disable_cob_delay);
    p.repeat = puc_params::opt_int(params, "repeat", p.repeat);
    p.thread_num = puc_params::opt_uint(params, "threadNum", p.thread_num);

    return refresh::result_to_json(config, p, refresh::simulate(config, p));
}
