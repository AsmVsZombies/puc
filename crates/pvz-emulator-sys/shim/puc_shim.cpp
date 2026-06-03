// FFI dispatcher. Deliberately includes NO seml headers — each calculator lives
// in its own TU (calc_*.cpp) to avoid the relative-include collision that occurs
// when several calculators' headers share a translation unit (a later calc's
// `#include "lib.h"` binds to an already-#pragma-once'd sibling). This TU only
// routes the call, wraps exceptions, and manages the malloc'd output buffer.

#include <cstdlib>
#include <cstring>
#include <exception>
#include <stdexcept>
#include <string>

#include "rapidjson/stringbuffer.h"
#include "rapidjson/writer.h"

#include "puc_shim.h"

// Defined in the per-calculator TUs. Each parses the scenario + params JSON,
// runs the simulation, and returns the result table as a JSON string; failures
// throw std::exception, caught below.
std::string puc_dispatch_pos(const char* scenario_json, const char* params_json);
std::string puc_dispatch_smash(const char* scenario_json, const char* params_json);
std::string puc_dispatch_explode(const char* scenario_json, const char* params_json);
std::string puc_dispatch_refresh(const char* scenario_json, const char* params_json);
std::string puc_dispatch_pogo(const char* scenario_json, const char* params_json);

namespace {

// {"error": "<msg>"} with proper JSON escaping.
std::string make_error_json(const std::string& msg)
{
    rapidjson::StringBuffer sb;
    rapidjson::Writer<rapidjson::StringBuffer> w(sb);
    w.StartObject();
    w.Key("error");
    w.String(msg.c_str());
    w.EndObject();
    return sb.GetString();
}

std::string dispatch(const std::string& calc, const char* scenario_json, const char* params_json)
{
    if (calc == "pos")
        return puc_dispatch_pos(scenario_json, params_json);
    if (calc == "smash")
        return puc_dispatch_smash(scenario_json, params_json);
    if (calc == "explode")
        return puc_dispatch_explode(scenario_json, params_json);
    if (calc == "refresh")
        return puc_dispatch_refresh(scenario_json, params_json);
    if (calc == "pogo")
        return puc_dispatch_pogo(scenario_json, params_json);
    throw std::runtime_error("unknown calculator: " + calc);
}

} // namespace

extern "C" int puc_run(const char* calculator, const char* scenario_json, const char* params_json,
    char** out_json)
{
    std::string result;
    int rc = 0;
    try {
        if (!calculator || !scenario_json)
            throw std::runtime_error("calculator and scenario_json must not be null");
        result = dispatch(calculator, scenario_json, params_json);
    } catch (const std::exception& e) {
        result = make_error_json(e.what());
        rc = 1;
    } catch (...) {
        result = make_error_json("unknown error");
        rc = 1;
    }

    if (out_json) {
        *out_json = static_cast<char*>(std::malloc(result.size() + 1));
        if (*out_json) {
            std::memcpy(*out_json, result.c_str(), result.size() + 1);
        } else {
            rc = 2; // allocation failure; out_json left as nullptr
        }
    }
    return rc;
}

extern "C" void puc_free(char* p) { std::free(p); }
