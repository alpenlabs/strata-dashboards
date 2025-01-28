import { lazy, Suspense } from "react";
import { useNetworkStatus } from "../hooks/useNetworkStatus";

const StatusCard = lazy(() => import("../components/StatusCard"));

export default function Dashboard() {
    const { data, isLoading, error } = useNetworkStatus();
    console.log("data", data);
    if (error) return <p className="text-red-500">Error loading data</p>;

    return (
        <div className="min-h-screen flex flex-col items-center p-10">
            <h1 className="text-3xl font-bold mb-6">Network Monitor</h1>
            <div className="w-full max-w-2xl bg-gray-800 p-6 rounded-lg shadow-lg">
                <Suspense fallback={<p className="text-white">Loading...</p>}>
                    {isLoading ? (
                        <p className="text-white">Loading...</p>
                    ) : (
                        <>
                            <StatusCard title="Batch Producer Status" status={data?.batch_producer ?? 'Unknown'} />
                            <StatusCard title="RPC Endpoint Status" status={data?.rpc_endpoint ?? "Unknown"} />
                            <StatusCard title="Bundler Endpoint Status" status={data?.bundler_endpoint ?? "Unknown"} />
                        </>
                    )}
                </Suspense>
            </div>
        </div>
    );
}