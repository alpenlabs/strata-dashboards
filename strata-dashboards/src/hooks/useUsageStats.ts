import { useQuery } from "@tanstack/react-query";

export type Account = {
    address: string,
    deployed_at: string,
    gas_used: string,
}

export type UsageStats = {
    stats: Record<string, Record<string, number>>,
    sel_accounts: Record<string, Record<string, Account[]>>,
};

const API_BASE_URL = import.meta.env.API_BASE_URL || "http://localhost:3000";
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
        refetchInterval: 60000, // ✅ Auto-refetch every 60s
    });
};
