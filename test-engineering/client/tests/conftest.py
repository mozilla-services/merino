# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import os
import pathlib

import yaml
from models import Scenario


def pytest_configure(config):
    """Load test scenarios from file."""

    scenarios_file = os.environ["SCENARIOS_FILE"]

    with pathlib.Path(scenarios_file).open() as f:
        loaded_scenarios = yaml.safe_load(f)

    """    config.merino_scenarios = [
        Scenario(**scenario) for scenario in loaded_scenarios["scenarios"]
    ]
    
    """
    scenarios = []
    for scenario in loaded_scenarios["scenarios"]:
        scenario_obj = Scenario(**scenario)
        assert( "client_variants" in scenario_obj.steps[0].response.content.dict())
        scenarios.append(scenario_obj)
    config.merino_scenarios = scenarios


def pytest_generate_tests(metafunc):
    """Generate tests from the loaded test scenarios."""

    ids = []
    argvalues = []

    for scenario in metafunc.config.merino_scenarios:
        ids.append(scenario.name)
        argvalues.append([scenario.steps])

    metafunc.parametrize(["steps"], argvalues, ids=ids)


def pytest_addoption(parser):
    """Define custom CLI options."""
    merino_group = parser.getgroup("merino")
    merino_group.addoption(
        "--merino-url",
        action="store",
        dest="merino_url",
        help="Merino endpoint URL",
        metavar="MERINO_URL",
        default=os.environ.get("MERINO_URL"),
        type=str,
        required=False,
    )
