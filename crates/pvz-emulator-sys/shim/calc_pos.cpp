// pos calculator dispatch — its own TU so the relative includes in pos/run.h
// (and the seml headers it pulls in) never collide with another calculator's.

#include "rapidjson/document.h"

#include "seml/pos/run.h"

#include "params.h"

std::string puc_dispatch_pos(const char* scenario_json, const char* params_json)
{
    using pvz_emulator::object::zombie_type;

    Config config = read_json_string(scenario_json);
    auto params = puc_params::parse(params_json);

    pos::RunParams p;
    for (int n : puc_params::opt_int_array(params, "zombies")) {
        p.zombie_types.push_back(static_cast<zombie_type>(n));
    }
    p.repeat = puc_params::opt_int(params, "repeat", p.repeat);
    p.target_x = puc_params::opt_int(params, "targetX", p.target_x);
    p.disable_cob_delay = puc_params::opt_bool(params, "disableCobDelay", p.disable_cob_delay);
    p.huge = puc_params::opt_bool(params, "huge", p.huge);
    p.thread_num = puc_params::opt_uint(params, "threadNum", p.thread_num);

    return pos::result_to_json(config, p, pos::simulate(config, p));
}
