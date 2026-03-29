"use client";

import Card from "@/components/ui/card";
import StatusBadge from "@/components/ui/statusBadge";
import useSWR from "swr";

const fetcher = (url: string) => fetch(url).then(res => res.json());

export default function Page() {
  let isHubConnected: boolean;

  const { data, error } = useSWR("http://localhost:3001/api/status", fetcher, { refreshInterval: 50000 });
  
  if (error) return <div>Error loading data</div>;
  if (!data) return <div>Loading...</div>;
  
  isHubConnected = data.isConnected;

  return (
    <div className="grid grid-cols-1 gap-8">
      <Card>
        <StatusBadge status={isHubConnected ? "ok" : "err"} title="Hub Connected" />
        <StatusBadge status="err" title="Battery Level" />
      </Card>
      <Card>Match Timer</Card>
      <Card>Logs</Card>
    </div>
  );
}