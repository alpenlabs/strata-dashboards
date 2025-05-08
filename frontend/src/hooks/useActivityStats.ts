import { useQuery } from "@tanstack/react-query";
import { useConfig } from "../hooks/useConfig";

export type Account = {
    address: string;
    creation_timestamp: string;
    gas_used: string;
};

export type ActivityStats = {
    stats: Record<string, Record<string, number>>;
    selected_accounts: Record<string, Account[]>;
};

const fetchActivityStats = async (baseUrl: string): Promise<ActivityStats> => {
    const response = await fetch(`${baseUrl}/api/activity_stats`);
    if (!response.ok) {
        throw new Error("Failed to fetch activity stats");
    }
    return response.json();
};

export const useActivityStats = () => {
    const { apiBaseUrl, activityStatsRefetchIntervalS } = useConfig();
    return useQuery({
        queryKey: ["activityStats"],
        queryFn: () => fetchActivityStats(apiBaseUrl),
        refetchInterval: activityStatsRefetchIntervalS * 1000, // convert to ms
    });
};
