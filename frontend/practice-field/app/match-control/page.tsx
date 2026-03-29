// dev note: this page is just a test of layour, functionality will need to be re-done in the future

"use client";

import { useState } from "react";
import Card from "@/components/ui/card";
import Button from "@/components/ui/button";
import StatusBadge from "@/components/ui/statusBadge";
import Checkbox from "@/components/ui/checkbox";
import Input from "@/components/ui/input";

export default function Page() {
    // match controls
    const [matchRunning, setMatchRunning] = useState(false);
    const [timeLeft, setTimeLeft] = useState("00:20");

    // team status
    const [bypassed, setBypassed] = useState([false, false, false, false, false, false]);
    const [radioStatus, setRadioStatus] = useState(["err", "err", "err", "err", "err", "err"]);
    const [driverstationStatus, setDriverstationStatus] = useState(["err", "err", "err", "err", "err", "err"]);
    const [rioStatus, setRioStatus] = useState(["err", "err", "err", "err", "err", "err"]);

    // match timing
    const [autoTime, setAutoTime] = useState(20);
    const [transitionShiftTime, setTransitionShiftTime] = useState(10);
    const [shift1Time, setShift1Time] = useState(25);
    const [shift2Time, setShift2Time] = useState(25);
    const [shift3Time, setShift3Time] = useState(25);
    const [shift4Time, setShift4Time] = useState(25);
    const [endgameTime, setEndgameTime] = useState(30);

    // score controls
    const [redScore, setRedScore] = useState(0);
    const [blueScore, setBlueScore] = useState(0);

    // network status
    const [apStatus, setApStatus] = useState("err");
    const [redHubStatus, setRedHubStatus] = useState("err");
    const [blueHubStatus, setBlueHubStatus] = useState("err");
    const [displayPiStatus, setDisplayPiStatus] = useState("err");

    let startMatchColor = "#6c757d";
    let stopMatchColor = "#6c757d";

    const allStationsReady = bypassed.every((isBypassed, index) => {
        if (isBypassed) return true;
        return (
            radioStatus[index] === "ok" &&
            driverstationStatus[index] === "ok" &&
            rioStatus[index] === "ok"
        );
    });

    const networkReady = [apStatus, redHubStatus, blueHubStatus, displayPiStatus].every(status => status === "ok");

    if (matchRunning) {
        startMatchColor = "#6c757d";
        stopMatchColor = "#dc3545";
    } else {
        startMatchColor = (allStationsReady && networkReady) ? "#28a745" : "#6c757d";
        stopMatchColor = "#6c757d";
    }

    function onStartMatch() {
        if (allStationsReady && networkReady) {
            setMatchRunning(true);
            setTimeLeft("01:00");
        }
    }

    function onStopMatch() {
        setMatchRunning(false);
        setTimeLeft("00:20");
    }

    function onBypassTeam(index: number, checked: boolean) {
        const newBypassed = [...bypassed];
        newBypassed[index] = checked;
        setBypassed(newBypassed);

        const newRadioStatus = [...radioStatus];
        newRadioStatus[index] = checked ? "warn" : "err";
        setRadioStatus(newRadioStatus);

        const newDriverstationStatus = [...driverstationStatus];
        newDriverstationStatus[index] = checked ? "warn" : "err";
        setDriverstationStatus(newDriverstationStatus);

        const newRioStatus = [...rioStatus];
        newRioStatus[index] = checked ? "warn" : "err";
        setRioStatus(newRioStatus);
    }

    return (
        <div className="grid grid-cols-1 gap-8">
            <Card>
                <div className="flex flex-col items-center gap-4">
                    <div className="flex items-center gap-16">
                        <Button text="Start Match" color={startMatchColor} onPress={onStartMatch} />
                        <a className="text-4xl">{timeLeft}</a>
                        <Button text="Stop Match" color={stopMatchColor} onPress={onStopMatch} />
                    </div>
                </div>
            </Card>

            <Card>
                <div className="grid grid-cols-2 grid-rows-1 gap-8">
                    <div className="grid grid-cols-3 grid-rows-5 gap-3 bg-red-900 p-6 rounded-xl">
                        <h1 className="text-xl font-bold">Red 1</h1>
                        <h1 className="text-xl font-bold">Red 2</h1>
                        <h1 className="text-xl font-bold">Red 3</h1>

                        <Checkbox checked={bypassed[0]} checkedColor="#aaaa00" label="Bypass" onChange={(checked) => onBypassTeam(0, checked)} />
                        <Checkbox checked={bypassed[1]} checkedColor="#aaaa00" label="Bypass" onChange={(checked) => onBypassTeam(1, checked)} />
                        <Checkbox checked={bypassed[2]} checkedColor="#aaaa00" label="Bypass" onChange={(checked) => onBypassTeam(2, checked)} />

                        <StatusBadge status={radioStatus[0]} title="Radio" />
                        <StatusBadge status={radioStatus[1]} title="Radio" />
                        <StatusBadge status={radioStatus[2]} title="Radio" />

                        <StatusBadge status={driverstationStatus[0]} title="Driverstation" />
                        <StatusBadge status={driverstationStatus[1]} title="Driverstation" />
                        <StatusBadge status={driverstationStatus[2]} title="Driverstation" />

                        <StatusBadge status={rioStatus[0]} title="Rio" />
                        <StatusBadge status={rioStatus[1]} title="Rio" />
                        <StatusBadge status={rioStatus[2]} title="Rio" />
                    </div>

                    <div className="grid grid-cols-3 grid-rows-5 gap-3 bg-blue-900 p-6 rounded-xl">
                        <h1 className="text-xl font-bold">Blue 1</h1>
                        <h1 className="text-xl font-bold">Blue 2</h1>
                        <h1 className="text-xl font-bold">Blue 3</h1>

                        <Checkbox checked={bypassed[3]} checkedColor="#aaaa00" label="Bypass" onChange={(checked) => onBypassTeam(3, checked)} />
                        <Checkbox checked={bypassed[4]} checkedColor="#aaaa00" label="Bypass" onChange={(checked) => onBypassTeam(4, checked)} />
                        <Checkbox checked={bypassed[5]} checkedColor="#aaaa00" label="Bypass" onChange={(checked) => onBypassTeam(5, checked)} />

                        <StatusBadge status={radioStatus[3]} title="Radio" />
                        <StatusBadge status={radioStatus[4]} title="Radio" />
                        <StatusBadge status={radioStatus[5]} title="Radio" />

                        <StatusBadge status={driverstationStatus[3]} title="Driverstation" />
                        <StatusBadge status={driverstationStatus[4]} title="Driverstation" />
                        <StatusBadge status={driverstationStatus[5]} title="Driverstation" />

                        <StatusBadge status={rioStatus[3]} title="Rio" />
                        <StatusBadge status={rioStatus[4]} title="Rio" />
                        <StatusBadge status={rioStatus[5]} title="Rio" />
                    </div>
                </div>
            </Card>

            <Card>
                <div className="grid grid-cols-3 gap-16">
                    <div className="flex flex-col items-center border-muted border-2 rounded-xl p-4">
                        <a className="p-4">Match Timing</a>
                        <div className="grid grid-cols-2 place-items-center ">
                            <a>Auton Time</a>
                            <Input value={autoTime.toString()} onChange={(value) => setAutoTime(parseInt(value) || 0)} type="number" placeholder="Auton Time" />

                            <a>Transition Shift</a>
                            <Input value={transitionShiftTime.toString()} onChange={(value) => setTransitionShiftTime(parseInt(value) || 0)} type="number" placeholder="Transition Shift" />

                            <a>Shift 1</a>
                            <Input value={shift1Time.toString()} onChange={(value) => setShift1Time(parseInt(value) || 0)} type="number" placeholder="Shift 1 Time" />

                            <a>Shift 2</a>
                            <Input value={shift2Time.toString()} onChange={(value) => setShift2Time(parseInt(value) || 0)} type="number" placeholder="Shift 2 Time" />

                            <a>Shift 3</a>
                            <Input value={shift3Time.toString()} onChange={(value) => setShift3Time(parseInt(value) || 0)} type="number" placeholder="Shift 3 Time" />

                            <a>Shift 4</a>
                            <Input value={shift4Time.toString()} onChange={(value) => setShift4Time(parseInt(value) || 0)} type="number" placeholder="Shift 4 Time" />

                            <a>Endgame</a>
                            <Input value={endgameTime.toString()} onChange={(value) => setEndgameTime(parseInt(value) || 0)} type="number" placeholder="Endgame Time" />
                        </div>
                    </div>

                    <div className="flex flex-col items-center border-muted border-2 rounded-xl p-4">
                        <a className="p-4">Score Controls</a>
                        <div className="grid grid-cols-2 place-items-center ">
                            <a>Red Score</a>
                            <Input value={redScore.toString()} onChange={(value) => setRedScore(parseInt(value) || 0)} type="number" placeholder="Red Score" />

                            <a>Blue Score</a>
                            <Input value={blueScore.toString()} onChange={(value) => setBlueScore(parseInt(value) || 0)} type="number" placeholder="Blue Score" />
                        </div>
                    </div>

                    <div className="flex flex-col items-center border-muted border-2 rounded-xl p-4">
                        <a className="p-4">Network Status</a>
                        <div className="grid grid-cols-1 gap-4">
                            <StatusBadge status={apStatus} title="AP" />
                            <StatusBadge status={redHubStatus} title="Red Hub" />
                            <StatusBadge status={blueHubStatus} title="Blue Hub" />
                            <StatusBadge status={displayPiStatus} title="Display Pi" />
                        </div>
                    </div>
                </div>
            </Card>
        </div>
    );
}