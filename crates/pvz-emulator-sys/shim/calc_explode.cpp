// explode calculator dispatch — see calc_pos.cpp for the per-TU rationale.

#include "rapidjson/document.h"

#include "seml/explode/run.h"

#include "params.h"

std::string puc_dispatch_explode(const char* scenario_json, const char* params_json)
{
    Config config = read_json_string(scenario_json);
    auto params = puc_params::parse(params_json);

    explode::RunParams p;
    p.repeat = puc_params::opt_int(params, "repeat", p.repeat);
    p.disable_cob_delay = puc_params::opt_bool(params, "disableCobDelay", p.disable_cob_delay);
    p.thread_num = puc_params::opt_uint(params, "threadNum", p.thread_num);

    return explode::result_to_json(config, p, explode::simulate(config, p));
}
