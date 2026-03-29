"use client"

import Button from "@/components/ui/button"
import Card from "@/components/ui/card"
import Input from "@/components/ui/input"
import { use, useState } from "react"

export default function Page() {
    const [stations, setStations] = useState([["1", ""], ["2", ""], ["3", ""], ["4", ""], ["5", ""], ["6", ""]])

    function updateStation(index: number, team: string, wpakey: string) {
        const newStations = [...stations]
        newStations[index] = [team, wpakey]
        setStations(newStations)
    }


    return (
        <div>
            <Card>
                <a className="text-2xl font-bold p-6">Team Management</a>
                <div className="grid grid-cols-6 grid-rows-3 place-items-center gap-4 p-6">
                    <a className="font-bold">Station 1</a>
                    <a className="font-bold">Station 2</a>
                    <a className="font-bold">Station 3</a>
                    <a className="font-bold">Station 4</a>
                    <a className="font-bold">Station 5</a>
                    <a className="font-bold">Station 6</a>

                    <Input placeholder="1" value={stations[0][0].toString()} onChange={(val) => updateStation(0, val, stations[0][1])} />
                    <Input placeholder="2" value={stations[1][0].toString()} onChange={(val) => updateStation(1, val, stations[1][1])} />
                    <Input placeholder="3" value={stations[2][0].toString()} onChange={(val) => updateStation(2, val, stations[2][1])} />
                    <Input placeholder="4" value={stations[3][0].toString()} onChange={(val) => updateStation(3, val, stations[3][1])} />
                    <Input placeholder="5" value={stations[4][0].toString()} onChange={(val) => updateStation(4, val, stations[4][1])} />
                    <Input placeholder="6" value={stations[5][0].toString()} onChange={(val) => updateStation(5, val, stations[5][1])} />

                    <Input placeholder="WPA Key" value={stations[0][1].toString()} onChange={(val) => updateStation(0, stations[0][0], val)} />    
                    <Input placeholder="WPA Key" value={stations[1][1].toString()} onChange={(val) => updateStation(1, stations[1][0], val)} />
                    <Input placeholder="WPA Key" value={stations[2][1].toString()} onChange={(val) => updateStation(2, stations[2][0], val)} />
                    <Input placeholder="WPA Key" value={stations[3][1].toString()} onChange={(val) => updateStation(3, stations[3][0], val)} />
                    <Input placeholder="WPA Key" value={stations[4][1].toString()} onChange={(val) => updateStation(4, stations[4][0], val)} />
                    <Input placeholder="WPA Key" value={stations[5][1].toString()} onChange={(val) => updateStation(5, stations[5][0], val)} />
                </div>

                <div className="flex flex-row gap-4 p-6">
                    <Button text="Generate WPA Keys" color="#3b82f6" onPress={() => {
                        const newStations = stations.map(([team, wpakey]) => [team, Math.random().toString(36).slice(-8)]);
                        setStations(newStations);
                    }} />
                    <Button text="Save" color="#10b981" onPress={() => {
                        // Save logic here (e.g., send to backend)
                        console.log("Saved stations:", stations);
                    }} />
                </div>
            </Card>
        </div>
    )
}