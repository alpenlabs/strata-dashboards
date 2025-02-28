import { useQuery } from "@tanstack/react-query";

export type NetworkStatus = {
    batch_producer: string;
    rpc_endpoint: string;
    bundler_endpoint: string;
};

const VITE_API_BASE_URL = import.meta.env.VITE_API_BASE_URL || "http://localhost:3000";
const fetchStatus = async (): Promise<NetworkStatus> => {
    const response = await fetch(`${VITE_API_BASE_URL}/api/status`);
    if (!response.ok) {
        throw new Error("Failed to fetch status");
    }
    return response.json();
};

export const useNetworkStatus = () => {
    return useQuery({
        queryKey: ["networkStatus"],
        queryFn: fetchStatus,
        refetchInterval: 10000, // ✅ Auto-refetch every 30s
    });
};