"use client"

import Arrow from "@/components/ui/arrow"



export default function Page() {
    return (
        <div className="flex flex-col items-center">
            <a className="text-[280pt] font-bold tracking-wide font-mono p-8">0:00</a>
            <div className="flex flex-row items-center gap-18">
                <a className="text-[100pt] font-bold tracking-wide font-mono">00</a>
                <Arrow direction="left" size={150} color="red" />
                <a className="text-[100pt] font-bold tracking-wide font-mono">AUTO</a>
                <Arrow direction="right" size={150} color="blue" />
                <a className="text-[100pt] font-bold tracking-wide font-mono">00</a>
            </div>
        </div>
    )
}