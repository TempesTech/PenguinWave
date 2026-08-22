export interface Query<T>{
    queryKey:string[],
    queryFn: (...args: any[]) => Promise<T>
}