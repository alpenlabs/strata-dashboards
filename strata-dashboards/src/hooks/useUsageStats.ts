import { useQuery } from "@tanstack/react-query";

export type Account = {
    address: string,
    creation_timestamp: string,
    gas_used: string,
}

export type UsageStats = {
    stats: Record<string, Record<string, number>>,
    selected_accounts: Record<string, Account[]>,
};

const API_BASE_URL = import.meta.env.API_BASE_URL || "http://localhost:3000";
// Default 120000 (2 minutes)
const REFETCH_INTERVAL = parseInt(import.meta.env.USAGE_STATS_FRONTEND_REFETCH_INTERVAL) || 120;
const fetchUsageStats = async (): Promise<UsageStats> => {
    const response = await fetch(`${API_BASE_URL}/api/usage_stats`);
    if (!response.ok) {
        throw new Error("Failed to fetch usage stats");
    }
    return response.json();
};

export const useUsageStats = () => {
    return useQuery({
        queryKey: ["usageStats"],
        queryFn: fetchUsageStats,
        // REFETCH_INTERVAL is in seconds
        refetchInterval: REFETCH_INTERVAL * 1000,
    });
};
