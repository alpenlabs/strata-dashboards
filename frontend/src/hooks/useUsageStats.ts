import { useQuery } from "@tanstack/react-query";

export type Account = {
    address: string;
    creation_timestamp: string;
    gas_used: string;
};

export type UsageStats = {
    stats: Record<string, Record<string, number>>;
    selected_accounts: Record<string, Account[]>;
};

const VITE_API_BASE_URL =
    import.meta.env.VITE_API_BASE_URL || "http://localhost:3000";
const REFETCH_INTERVAL_S =
    parseInt(import.meta.env.VITE_USAGE_STATS_REFETCH_INTERVAL_S) || 120;

const fetchUsageStats = async (): Promise<UsageStats> => {
    const response = await fetch(`${VITE_API_BASE_URL}/api/usage_stats`);
    if (!response.ok) {
        throw new Error("Failed to fetch usage stats");
    }
    return response.json();
};

export const useUsageStats = () => {
    return useQuery({
        queryKey: ["usageStats"],
        queryFn: fetchUsageStats,
        refetchInterval: REFETCH_INTERVAL_S * 1000,
    });
};
